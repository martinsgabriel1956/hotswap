//! No AppImage, o AppRun aponta LD_LIBRARY_PATH, PATH, PYTHONHOME etc. para dentro da imagem. O próprio app e o WebKit
//! precisam disso, mas os programas do usuário quebram com isso (o python do sistema nem inicia). Tudo o que o app abre
//! passa por aqui. Fora do AppImage, não muda nada.
use std::ffi::{OsStr, OsString};
use std::path::Path;
use std::process::Command;

/// Só existem por causa do AppImage, mas o valor não aponta para dentro dele.
const APPIMAGE_ONLY: [&str; 6] = ["APPDIR", "APPIMAGE", "ARGV0", "OWD", "GTK_THEME", "PYTHONDONTWRITEBYTECODE"];

/// O que mudar no ambiente de um processo filho: `Some` troca o valor, `None` remove a variável.
pub fn overrides() -> Vec<(OsString, Option<String>)> {
    match std::env::var("APPDIR") {
        Ok(appdir) if !appdir.is_empty() => overrides_for(&appdir, std::env::vars_os()),
        _ => Vec::new(),
    }
}

fn overrides_for(appdir: &str, vars: impl Iterator<Item = (OsString, OsString)>) -> Vec<(OsString, Option<String>)> {
    vars.filter_map(|(key, value)| {
        if APPIMAGE_ONLY.iter().any(|name| key == *name) {
            return Some((key, None));
        }
        let value = value.to_str()?;
        if !value.contains(appdir) {
            return None;
        }
        // Tira também as entradas vazias: o AppRun deixa um ":" no fim, e no LD_LIBRARY_PATH ou no PATH isso é a pasta atual.
        let kept: Vec<&str> = value.split(':').filter(|entry| !entry.is_empty() && !Path::new(entry).starts_with(appdir)).collect();
        Some((key, (!kept.is_empty()).then(|| kept.join(":"))))
    })
    .collect()
}

/// Aberto pelo Finder, o app herda o PATH mínimo do launchd, sem Homebrew nem nvm: `gh` e `npm` não seriam achados.
/// Adota o PATH do shell do usuário; o printf vai depois de qualquer coisa que o .zshrc escreva.
/// ponytail: um shell que trava ao iniciar sem terminal trava a abertura do app; pôr timeout se aparecer.
#[cfg(target_os = "macos")]
pub fn adopt_shell_path() {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".into());
    let out = Command::new(shell).args(["-ilc", r#"printf '\n%s' "$PATH""#]).stdin(std::process::Stdio::null()).output();
    if let Ok(out) = out {
        let stdout = String::from_utf8_lossy(&out.stdout);
        if let Some(path) = stdout.lines().last().filter(|p| !p.is_empty()) {
            std::env::set_var("PATH", path);
        }
    }
}

/// `Command::new` para programas do usuário, sem o ambiente do AppImage.
pub fn command(program: impl AsRef<OsStr>) -> Command {
    let mut cmd = Command::new(program);
    // O app roda dentro da imagem porque o WebKit é achado por caminho relativo; o filho volta para onde o app foi aberto.
    // Sem isso, um editor que sobrevive ao app ficaria numa pasta desmontada.
    if let (Ok(_), Ok(owd)) = (std::env::var("APPDIR"), std::env::var("OWD")) {
        if Path::new(&owd).is_dir() {
            cmd.current_dir(owd);
        }
    }
    for (key, value) in overrides() {
        match value {
            Some(value) => cmd.env(key, value),
            None => cmd.env_remove(key),
        };
    }
    cmd
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tira_so_o_que_veio_do_appimage() {
        let vars = [
            ("APPDIR", "/tmp/.mount_x"),
            ("PATH", "/tmp/.mount_x/usr/bin/:/tmp/.mount_x/bin/:/usr/bin:/bin"),
            ("LD_LIBRARY_PATH", "/tmp/.mount_x/usr/lib/:"),
            ("PYTHONHOME", "/tmp/.mount_x/usr/"),
            ("GTK_PATH", "/tmp/.mount_x//usr/lib/gtk-3.0"),
            ("PYTHONDONTWRITEBYTECODE", "1"),
            ("VIZINHO", "/tmp/.mount_xyz/bin:/opt/bin"),
            ("HOME", "/home/eu"),
        ];
        let got = overrides_for("/tmp/.mount_x", vars.into_iter().map(|(k, v)| (k.into(), v.into())));
        let get = |name: &str| got.iter().find(|(k, _)| k == name).map(|(_, v)| v.as_deref());

        assert_eq!(get("APPDIR"), Some(None));
        assert_eq!(get("PATH"), Some(Some("/usr/bin:/bin")));
        assert_eq!(get("LD_LIBRARY_PATH"), Some(None));
        assert_eq!(get("PYTHONHOME"), Some(None));
        assert_eq!(get("GTK_PATH"), Some(None));
        assert_eq!(get("PYTHONDONTWRITEBYTECODE"), Some(None));
        // Pasta com nome parecido com o da imagem não é a imagem.
        assert_eq!(get("VIZINHO"), Some(Some("/tmp/.mount_xyz/bin:/opt/bin")));
        assert_eq!(get("HOME"), None);
    }
}
