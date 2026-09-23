//! Two faces for one restore flow: plain terminal prompts, or real windows
//! via `zenity` (installed by default on Ubuntu desktop) when the user
//! double-clicked the launcher and there is no terminal at all.

use anyhow::{bail, Context, Result};
use std::cell::RefCell;
use std::io::{IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};

pub enum Ui {
    Term,
    Gui,
}

pub fn has(program: &str) -> bool {
    Command::new("sh")
        .args(["-c", "command -v \"$1\"", "sh", program])
        .stdout(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

/// zenity text is Pango markup; user-derived strings (file and app names)
/// must not be able to break it.
fn plain(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

fn zenity(args: &[&str]) -> Option<String> {
    let out = Command::new("zenity").arg("--title=Ferry").args(args).output().ok()?;
    out.status.success().then(|| String::from_utf8_lossy(&out.stdout).trim_end_matches('\n').to_string())
}

impl Ui {
    pub fn detect(args: &[String]) -> Ui {
        let wants_gui = args.iter().any(|a| a == "--gui") || !std::io::stdin().is_terminal();
        if wants_gui && !args.iter().any(|a| a == "--no-gui") && has("zenity") {
            Ui::Gui
        } else {
            Ui::Term
        }
    }

    pub fn say(&self, text: &str) {
        if let Ui::Term = self {
            println!("{text}");
        }
    }

    pub fn password(&self) -> Result<String> {
        let password = match self {
            Ui::Term => rpassword::prompt_password("Backup password: ").context("Could not read the password")?,
            Ui::Gui => zenity(&["--password"]).context("Cancelled")?,
        };
        if password.is_empty() {
            bail!("No password entered");
        }
        Ok(password)
    }

    /// Only the window version can ask; the terminal version takes a path argument.
    pub fn pick_folder(&self) -> Option<PathBuf> {
        match self {
            Ui::Term => None,
            Ui::Gui => zenity(&["--file-selection", "--directory", "--title=Ferry — select your Ferry USB drive"])
                .map(PathBuf::from),
        }
    }

    pub fn ask(&self, question: &str) -> bool {
        match self {
            Ui::Term => {
                print!("{question} [Y/n] ");
                std::io::stdout().flush().ok();
                let mut answer = String::new();
                std::io::stdin().read_line(&mut answer).is_ok()
                    && !answer.trim().to_lowercase().starts_with('n')
            }
            Ui::Gui => zenity(&["--question", "--width=420", "--no-markup", &format!("--text={question}")]).is_some(),
        }
    }

    /// Rows are (ticked by default, label). Returns the chosen row indices.
    pub fn checklist(&self, text: &str, rows: &[(bool, String)]) -> Vec<usize> {
        match self {
            Ui::Term => {
                println!("\n{text}");
                for (ticked, label) in rows {
                    println!("  [{}] {label}", if *ticked { "x" } else { " " });
                }
                if self.ask("Install the ticked apps? (the others are listed in linux-apps.md)") {
                    (0..rows.len()).filter(|&i| rows[i].0).collect()
                } else {
                    Vec::new()
                }
            }
            Ui::Gui => {
                let mut args: Vec<String> = [
                    "--list", "--checklist", "--width=560", "--height=440",
                    "--column=Install", "--column=#", "--column=App",
                    "--hide-column=2", "--print-column=2", "--separator=\n",
                ]
                .map(String::from)
                .to_vec();
                args.push(format!("--text={}", plain(text)));
                for (i, (ticked, label)) in rows.iter().enumerate() {
                    args.extend([if *ticked { "TRUE" } else { "FALSE" }.into(), i.to_string(), label.clone()]);
                }
                let refs: Vec<&str> = args.iter().map(String::as_str).collect();
                zenity(&refs)
                    .map(|out| out.lines().filter_map(|l| l.trim().parse().ok()).collect())
                    .unwrap_or_default()
            }
        }
    }

    /// `pulsate` for work with no known length (decrypting, installing).
    pub fn progress(&self, text: &str, pulsate: bool) -> Progress {
        let (child, stdin) = match self {
            Ui::Term => {
                println!("{text}");
                (None, None)
            }
            Ui::Gui => {
                let mut cmd = Command::new("zenity");
                cmd.args(["--title=Ferry", "--progress", "--width=460", "--no-cancel", "--auto-close"])
                    .arg(format!("--text={}", plain(text)));
                if pulsate {
                    cmd.arg("--pulsate");
                }
                match cmd.stdin(Stdio::piped()).spawn() {
                    Ok(mut child) => {
                        let stdin = child.stdin.take();
                        (Some(child), stdin)
                    }
                    Err(_) => (None, None),
                }
            }
        };
        Progress { child, stdin: RefCell::new(stdin) }
    }

    pub fn error(&self, text: &str) {
        match self {
            Ui::Term => eprintln!("\n{text}"),
            Ui::Gui => {
                zenity(&["--error", "--width=460", "--no-markup", &format!("--text={text}")]);
            }
        }
    }

    pub fn done(&self, lines: &[String], restored: &Path) {
        match self {
            Ui::Term => {
                println!();
                for line in lines {
                    println!("{line}");
                }
            }
            Ui::Gui => {
                let text = format!("Done — welcome to Ubuntu!\n\n{}", lines.join("\n\n"));
                zenity(&["--info", "--width=520", "--no-markup", &format!("--text={text}")]);
                let _ = Command::new("xdg-open").arg(restored).spawn();
            }
        }
    }
}

pub struct Progress {
    child: Option<Child>,
    stdin: RefCell<Option<ChildStdin>>,
}

impl Progress {
    pub fn set(&self, percent: u64, text: &str) {
        match self.stdin.borrow_mut().as_mut() {
            Some(pipe) => {
                let _ = write!(pipe, "{}\n# {}\n", percent.min(99), plain(text));
            }
            None if self.child.is_none() => {
                // \r keeps this on one line; truncation stops long names wrapping.
                print!("\r  {percent:>3}%  {text:<60.60}");
                std::io::stdout().flush().ok();
            }
            None => {}
        }
    }

    pub fn text(&self, text: &str) {
        match self.stdin.borrow_mut().as_mut() {
            Some(pipe) => {
                let _ = writeln!(pipe, "# {}", plain(text));
            }
            None => println!("  {text}"),
        }
    }
}

impl Drop for Progress {
    fn drop(&mut self) {
        self.stdin.borrow_mut().take();
        match self.child.take() {
            Some(mut child) => {
                let _ = child.kill();
                let _ = child.wait();
            }
            None => println!(),
        }
    }
}
