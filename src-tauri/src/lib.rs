mod host_env;
mod pty;
mod worktrees;

use std::path::Path;

use tauri::menu::{IsMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager, WindowEvent, Wry};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};

/// O app se chamava switch-terminal. Na primeira abertura como Hotswap, o config e os dados do webview
/// (última worktree por projeto, recentes, barra lateral) mudam para as pastas do nome novo.
fn migrate_from_switch_terminal() {
    let home = std::env::var("HOME").unwrap_or_default();
    let data = std::env::var("XDG_DATA_HOME").unwrap_or_else(|_| format!("{home}/.local/share"));
    move_if_new(
        Path::new(&format!("{home}/.config/switch-terminal")),
        Path::new(&format!("{home}/.config/hotswap")),
    );
    move_if_new(
        Path::new(&format!("{data}/dev.gabriel.switch-terminal")),
        Path::new(&format!("{data}/dev.gabriel.hotswap")),
    );
}

/// Move `old` para `new` só quando `new` ainda não existe: depois da primeira vez, não mexe em nada.
fn move_if_new(old: &Path, new: &Path) {
    if new.exists() || !old.exists() {
        return;
    }
    if let Err(e) = std::fs::rename(old, new) {
        eprintln!("não deu para mover {} para {}: {e}", old.display(), new.display());
    }
}

fn show_window(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

/// Sair de verdade encerra os dev servers, então pergunta antes se houver algo rodando.
fn quit(app: &AppHandle) {
    if !app.state::<pty::Ptys>().any_busy() {
        app.exit(0);
        return;
    }
    show_window(app);
    let handle = app.clone();
    app.dialog()
        .message("Há processos rodando nos terminais. Sair encerra todos eles.")
        .title("Sair do Hotswap")
        .kind(MessageDialogKind::Warning)
        .buttons(MessageDialogButtons::OkCancelCustom("Sair".into(), "Cancelar".into()))
        .show(move |ok| {
            if ok {
                handle.exit(0);
            }
        });
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
    let show = MenuItem::with_id(app, "show", "Mostrar janela", true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;
    let sep_end = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "Sair", true, None::<&str>)?;
    let mid = tray_items(app, entries)?;
    let mut refs: Vec<&dyn IsMenuItem<Wry>> = vec![&show, &sep];
    refs.extend(mid.iter().map(|m| m.as_ref()));
    if !mid.is_empty() {
        refs.push(&sep_end);
    }
    refs.push(&quit);
    Menu::with_items(app, &refs)
}

/// O front remonta o menu quando muda o que ele mostra (projetos, worktree atual, quem está de pé).
#[tauri::command]
fn set_tray_menu(app: AppHandle, entries: Vec<TrayEntry>) -> Result<(), String> {
    let menu = tray_menu(&app, &entries).map_err(|e| e.to_string())?;
    let tray = app.tray_by_id(TRAY_ID).ok_or("bandeja não criada")?;
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
    // Antes do Builder: o webview abre já lendo a pasta de dados do identifier novo.
    migrate_from_switch_terminal();
    tauri::Builder::default()
        // Abrir o app de novo só traz a janela existente: duas instâncias subiriam os dev servers em dobro.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| show_window(app)))
        .plugin(tauri_plugin_dialog::init())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn move_if_new_moves_once_and_never_overwrites() {
        let tmp = std::env::temp_dir().join(format!("hotswap-migrate-{}", std::process::id()));
        let old = tmp.join("old");
        let new = tmp.join("new");
        std::fs::create_dir_all(&old).unwrap();
        std::fs::write(old.join("config.json"), "antigo").unwrap();

        move_if_new(&old, &new);
        assert!(!old.exists());
        assert_eq!(std::fs::read_to_string(new.join("config.json")).unwrap(), "antigo");

        // O antigo reaparece (app velho ainda aberto): o novo continua valendo.
        std::fs::create_dir_all(&old).unwrap();
        std::fs::write(old.join("config.json"), "recriado").unwrap();
        move_if_new(&old, &new);
        assert_eq!(std::fs::read_to_string(new.join("config.json")).unwrap(), "antigo");
        assert!(old.exists());

        // Sem nada para mover, nada acontece.
        move_if_new(&tmp.join("nada"), &tmp.join("outro"));
        assert!(!tmp.join("outro").exists());

        std::fs::remove_dir_all(&tmp).unwrap();
    }
}
