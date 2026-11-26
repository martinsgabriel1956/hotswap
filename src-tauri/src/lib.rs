/// Idioma das mensagens que saem daqui (erros, bandeja, diálogo de sair): o front diz qual com set_lang ao abrir.
static EN: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
fn en() -> bool {
    EN.load(std::sync::atomic::Ordering::Relaxed)
}

/// `tr!("Já existe {}.", "{} already exists.", dest.display())`: o format! do idioma atual.
#[macro_export]
macro_rules! tr {
    ($pt:literal, $en:literal $(, $a:expr)* $(,)?) => {
        if $crate::en() { format!($en $(, $a)*) } else { format!($pt $(, $a)*) }
    };
}

mod host_env;
mod pty;
mod worktrees;

use tauri::menu::{IsMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager, WindowEvent, Wry};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};

fn show_window(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

/// Sair e reiniciar encerram os dev servers: com algo rodando, pergunta antes (diálogo nativo) e só então faz `then`.
fn confirm_end(app: &AppHandle, message: String, title: String, button: String, then: impl FnOnce(&AppHandle) + Send + 'static) {
    if !app.state::<pty::Ptys>().any_busy() {
        return then(app);
    }
    show_window(app);
    let handle = app.clone();
    app.dialog()
        .message(message)
        .title(title)
        .kind(MessageDialogKind::Warning)
        .buttons(MessageDialogButtons::OkCancelCustom(button, tr!("Cancelar", "Cancel")))
        .show(move |ok| {
            if ok {
                then(&handle);
            }
        });
}

fn quit(app: &AppHandle) {
    confirm_end(
        app,
        tr!("Há processos rodando nos terminais. Sair encerra todos eles.", "Processes are running in the terminals. Quitting ends all of them."),
        tr!("Sair do Hotswap", "Quit Hotswap"),
        tr!("Sair", "Quit"),
        |app| app.exit(0),
    );
}

/// Reinicia o app (versão instalada pelo update, idioma novo); os PTYs morrem antes, como ao sair.
#[tauri::command]
fn restart_app(app: AppHandle) {
    confirm_end(
        &app,
        tr!(
            "Há processos rodando nos terminais. Reiniciar o Hotswap encerra todos eles.",
            "Processes are running in the terminals. Restarting Hotswap ends all of them."
        ),
        tr!("Reiniciar o Hotswap", "Restart Hotswap"),
        tr!("Reiniciar", "Restart"),
        |app| {
            app.state::<pty::Ptys>().kill_all();
            app.restart()
        },
    );
}

/// Item do menu da bandeja montado pelo front, que sabe os projetos e quem está de pé.
#[derive(serde::Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
enum TrayEntry {
    Item { id: String, label: String, enabled: bool },
    Separator,
    Submenu { label: String, items: Vec<TrayEntry> },
}

/// Prefixo dos ids que vêm do front: o clique volta para ele como o evento "tray".
const UI_ID: &str = "ui:";

fn tray_items(app: &AppHandle, entries: &[TrayEntry]) -> tauri::Result<Vec<Box<dyn IsMenuItem<Wry>>>> {
    entries
        .iter()
        .map(|e| -> tauri::Result<Box<dyn IsMenuItem<Wry>>> {
            Ok(match e {
                TrayEntry::Item { id, label, enabled } => Box::new(MenuItem::with_id(app, format!("{UI_ID}{id}"), label, *enabled, None::<&str>)?),
                TrayEntry::Separator => Box::new(PredefinedMenuItem::separator(app)?),
                TrayEntry::Submenu { label, items } => {
                    let kids = tray_items(app, items)?;
                    let refs: Vec<&dyn IsMenuItem<Wry>> = kids.iter().map(|k| k.as_ref()).collect();
                    Box::new(Submenu::with_items(app, label, true, &refs)?)
                }
            })
        })
        .collect()
}

/// "Mostrar janela" e "Sair" ficam sempre; o que o front mandou vai entre eles.
fn tray_menu(app: &AppHandle, entries: &[TrayEntry]) -> tauri::Result<Menu<Wry>> {
    let show = MenuItem::with_id(app, "show", tr!("Mostrar janela", "Show window"), true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;
    let sep_end = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", tr!("Sair", "Quit"), true, None::<&str>)?;
    let mid = tray_items(app, entries)?;
    let mut refs: Vec<&dyn IsMenuItem<Wry>> = vec![&show, &sep];
    refs.extend(mid.iter().map(|m| m.as_ref()));
    if !mid.is_empty() {
        refs.push(&sep_end);
    }
    refs.push(&quit);
    Menu::with_items(app, &refs)
}

#[tauri::command]
fn set_lang(lang: String) {
    EN.store(lang == "en", std::sync::atomic::Ordering::Relaxed);
}

/// Atualiza pelo release do GitHub (latest.json): "none" se não há versão nova para este pacote (o front abre o release),
/// "installed" quando instalou; a nova vale ao reiniciar (restart_app), quando a pessoa escolher. No Windows o instalador
/// fecha o app e não volta.
#[tauri::command]
async fn install_update(app: AppHandle) -> Result<&'static str, String> {
    use tauri_plugin_updater::UpdaterExt;
    let Some(update) = app.updater().map_err(|e| e.to_string())?.check().await.map_err(|e| e.to_string())? else {
        return Ok("none");
    };
    update.download_and_install(|_, _| {}, || {}).await.map_err(|e| e.to_string())?;
    Ok("installed")
}

/// O front remonta o menu quando muda o que ele mostra (projetos, worktree atual, quem está de pé).
#[tauri::command]
fn set_tray_menu(app: AppHandle, entries: Vec<TrayEntry>) -> Result<(), String> {
    let menu = tray_menu(&app, &entries).map_err(|e| e.to_string())?;
    let tray = app.tray_by_id(TRAY_ID).ok_or(tr!("bandeja não criada", "tray not created"))?;
    tray.set_menu(Some(menu)).map_err(|e| e.to_string())
}

// No Linux o id vira o arquivo do ícone ($XDG_RUNTIME_DIR/tray-icon/tray-icon-<id>-0.png) e o id do
// AppIndicator: com "main", outro app Tauri com o mesmo id sobrescreve o arquivo e as duas bandejas
// mostram o ícone de quem abriu por último.
const TRAY_ID: &str = "hotswap";

fn setup_tray(app: &AppHandle) -> tauri::Result<()> {
    let menu = tray_menu(app, &[])?;
    let mut tray = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip("Hotswap")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => show_window(app),
            "quit" => quit(app),
            // Toda ação do front traz a janela: a troca pode pedir portas, e o resultado aparece nela.
            id => {
                if let Some(action) = id.strip_prefix(UI_ID) {
                    show_window(app);
                    let _ = app.emit("tray", action);
                }
            }
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(target_os = "macos")]
    host_env::adopt_shell_path();
    tauri::Builder::default()
        // Abrir o app de novo só traz a janela existente: duas instâncias subiriam os dev servers em dobro.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| show_window(app)))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(pty::Ptys::default())
        .setup(|app| Ok(setup_tray(app.handle())?))
        // O X só esconde na bandeja; os dev servers continuam rodando. Sair é pelo menu da bandeja.
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            set_lang,
            install_update,
            restart_app,
            set_tray_menu,
            pty::pty_spawn,
            pty::pty_write,
            pty::pty_resize,
            pty::pty_kill,
            pty::pty_stop,
            pty::pty_kill_all,
            pty::pty_force,
            pty::pty_status,
            pty::ports_open,
            pty::open_url,
            pty::notify,
            worktrees::load_config,
            worktrees::add_worktree,
            worktrees::worktree_defaults,
            worktrees::add_project,
            worktrees::open_config,
            worktrees::open_in_editor,
            worktrees::list_editors,
            worktrees::set_editor,
            worktrees::set_screen_reader,
            worktrees::set_parallel,
            worktrees::missing_deps_install,
            worktrees::lockfile_pm,
            worktrees::worktree_info,
            worktrees::edit_worktree,
            worktrees::remove_worktree,
            worktrees::list_worktrees,
            worktrees::git_status,
            worktrees::pull_worktree,
            worktrees::github_repos,
            worktrees::pull_requests,
            worktrees::open_pr,
            worktrees::clone_github_repo,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            if let tauri::RunEvent::Exit = event {
                app.state::<pty::Ptys>().kill_all();
            }
            // No macOS, abrir o app de novo (ou clicar no Dock) não cria outra instância: chega aqui.
            #[cfg(target_os = "macos")]
            if let tauri::RunEvent::Reopen { .. } = event {
                show_window(app);
            }
        });
}
