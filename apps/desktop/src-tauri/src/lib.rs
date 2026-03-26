use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Serialize)]
struct DesktopProfile {
  app_name: String,
  runtime: String,
  action_mode: String,
  notes: Vec<String>,
}

#[derive(Serialize)]
struct ActionItem {
  id: String,
  title: String,
  detail: String,
  state: String,
}

#[derive(Serialize, Deserialize, Clone)]
struct LocalAction {
  kind: String,
  target: String,
  label: String,
}

#[derive(Serialize)]
struct CommandPlan {
  assistant_reply: String,
  risk_level: String,
  can_execute_directly: bool,
  steps: Vec<ActionItem>,
  suggested_action: Option<LocalAction>,
}

#[derive(Serialize)]
struct ActionExecutionResult {
  ok: bool,
  summary: String,
  details: Vec<String>,
}

#[tauri::command]
fn get_desktop_profile() -> DesktopProfile {
  DesktopProfile {
    app_name: "xixi".into(),
    runtime: "tauri-desktop".into(),
    action_mode: "small_tasks_direct_large_tasks_confirm".into(),
    notes: vec![
      "Desktop shell is live".into(),
      "Only real desktop actions are exposed".into(),
      "Unsupported commands are reported instead of faked".into(),
    ],
  }
}

#[tauri::command]
fn plan_user_request(request: String) -> CommandPlan {
  let lowered = request.to_lowercase();

  if contains_any(&lowered, &["open qmdownload", "qmdownload", "download folder"]) {
    return direct_plan(
      "I can open the QMDownload folder right now.",
      vec![
        step("plan-qm-1", "Match command", "Mapped request to QMDownload folder", "done"),
        step("plan-qm-2", "Run folder open", "Open D:\\QMDownload in Explorer", "ready"),
      ],
      LocalAction {
        kind: "open_folder".into(),
        target: r"D:\QMDownload".into(),
        label: "QMDownload folder".into(),
      },
    );
  }

  if contains_any(&lowered, &["open xixi folder", "open xixi project", "xixi folder"]) {
    return direct_plan(
      "I can open the xixi project folder right now.",
      vec![
        step("plan-xixi-1", "Match command", "Mapped request to xixi project folder", "done"),
        step("plan-xixi-2", "Run folder open", "Open D:\\QMDownload\\xixi in Explorer", "ready"),
      ],
      LocalAction {
        kind: "open_folder".into(),
        target: r"D:\QMDownload\xixi".into(),
        label: "xixi project folder".into(),
      },
    );
  }

  if contains_any(&lowered, &["open github", "github"]) {
    return direct_plan(
      "I can open GitHub in your default browser right now.",
      vec![
        step("plan-gh-1", "Match command", "Mapped request to GitHub URL", "done"),
        step("plan-gh-2", "Run browser open", "Open https://github.com", "ready"),
      ],
      LocalAction {
        kind: "open_url".into(),
        target: "https://github.com".into(),
        label: "GitHub".into(),
      },
    );
  }

  if contains_any(&lowered, &["open weather", "weather"]) {
    return direct_plan(
      "I can open a weather search page right now.",
      vec![
        step("plan-weather-1", "Match command", "Mapped request to weather search URL", "done"),
        step(
          "plan-weather-2",
          "Run browser open",
          "Open the weather search page in the default browser",
          "ready",
        ),
      ],
      LocalAction {
        kind: "open_url".into(),
        target: "https://www.bing.com/search?q=today+weather".into(),
        label: "weather search".into(),
      },
    );
  }

  if contains_any(&lowered, &["open chrome", "chrome"]) {
    return direct_plan(
      "I can try to launch Google Chrome right now.",
      vec![
        step("plan-chrome-1", "Match command", "Mapped request to Chrome launch", "done"),
        step("plan-chrome-2", "Run app launch", "Try known Chrome executable locations", "ready"),
      ],
      LocalAction {
        kind: "open_app".into(),
        target: "chrome".into(),
        label: "Google Chrome".into(),
      },
    );
  }

  if contains_any(&lowered, &["open edge", "edge"]) {
    return direct_plan(
      "I can try to launch Microsoft Edge right now.",
      vec![
        step("plan-edge-1", "Match command", "Mapped request to Edge launch", "done"),
        step("plan-edge-2", "Run app launch", "Try known Edge executable locations", "ready"),
      ],
      LocalAction {
        kind: "open_app".into(),
        target: "edge".into(),
        label: "Microsoft Edge".into(),
      },
    );
  }

  if contains_any(&lowered, &["open notepad", "notepad"]) {
    return direct_plan(
      "I can launch Notepad right now.",
      vec![
        step("plan-notepad-1", "Match command", "Mapped request to Notepad launch", "done"),
        step("plan-notepad-2", "Run app launch", "Start notepad.exe", "ready"),
      ],
      LocalAction {
        kind: "open_app".into(),
        target: "notepad".into(),
        label: "Notepad".into(),
      },
    );
  }

  if contains_any(&lowered, &["open explorer", "file explorer", "explorer"]) {
    return direct_plan(
      "I can launch File Explorer right now.",
      vec![
        step("plan-explorer-1", "Match command", "Mapped request to Explorer launch", "done"),
        step("plan-explorer-2", "Run app launch", "Start explorer.exe", "ready"),
      ],
      LocalAction {
        kind: "open_app".into(),
        target: "explorer".into(),
        label: "File Explorer".into(),
      },
    );
  }

  unsupported_plan()
}

#[tauri::command]
fn execute_local_action(action: LocalAction) -> Result<ActionExecutionResult, String> {
  match action.kind.as_str() {
    "open_folder" => open_folder(&action.target, &action.label),
    "open_url" => open_url(&action.target, &action.label),
    "open_app" => open_app(&action.target, &action.label),
    other => Err(format!("Unsupported action kind: {other}")),
  }
}

fn direct_plan(reply: &str, steps: Vec<ActionItem>, action: LocalAction) -> CommandPlan {
  CommandPlan {
    assistant_reply: reply.into(),
    risk_level: "low-risk".into(),
    can_execute_directly: true,
    steps,
    suggested_action: Some(action),
  }
}

fn unsupported_plan() -> CommandPlan {
  CommandPlan {
    assistant_reply: "This command is not implemented yet. I will not pretend to run it.".into(),
    risk_level: "not-implemented".into(),
    can_execute_directly: false,
    steps: vec![
      step("plan-unsupported-1", "Read request", "Parsed the request text", "done"),
      step(
        "plan-unsupported-2",
        "Check registry",
        "No real desktop action is wired for this command yet",
        "done",
      ),
      step(
        "plan-unsupported-3",
        "Stop honestly",
        "Execution is blocked until a real adapter exists",
        "waiting",
      ),
    ],
    suggested_action: None,
  }
}

fn open_folder(target: &str, label: &str) -> Result<ActionExecutionResult, String> {
  Command::new("explorer")
    .arg(target)
    .spawn()
    .map_err(|error| format!("Failed to open folder: {error}"))?;

  Ok(ActionExecutionResult {
    ok: true,
    summary: format!("Opened {label}."),
    details: vec![target.into(), "Executed through Windows Explorer".into()],
  })
}

fn open_url(target: &str, label: &str) -> Result<ActionExecutionResult, String> {
  Command::new("cmd")
    .args(["/C", "start", "", target])
    .spawn()
    .map_err(|error| format!("Failed to open url: {error}"))?;

  Ok(ActionExecutionResult {
    ok: true,
    summary: format!("Opened {label}."),
    details: vec![target.into(), "Executed through the default browser".into()],
  })
}

fn open_app(target: &str, label: &str) -> Result<ActionExecutionResult, String> {
  let launched = match target {
    "chrome" => try_spawn_any(&[
      r"C:\Program Files\Google\Chrome\Application\chrome.exe",
      r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
    ]),
    "edge" => try_spawn_any(&[
      r"C:\Program Files\Microsoft\Edge\Application\msedge.exe",
      r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
    ]),
    "notepad" => Command::new("notepad.exe").spawn().is_ok(),
    "explorer" => Command::new("explorer.exe").spawn().is_ok(),
    _ => false,
  };

  if !launched {
    return Err(format!("Failed to launch {label}."));
  }

  Ok(ActionExecutionResult {
    ok: true,
    summary: format!("Launched {label}."),
    details: vec![format!("target={target}")],
  })
}

fn try_spawn_any(candidates: &[&str]) -> bool {
  candidates
    .iter()
    .any(|candidate| Command::new(candidate).spawn().is_ok())
}

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
  needles.iter().any(|needle| haystack.contains(needle))
}

fn step(id: &str, title: &str, detail: &str, state: &str) -> ActionItem {
  ActionItem {
    id: id.into(),
    title: title.into(),
    detail: detail.into(),
    state: state.into(),
  }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .setup(|app| {
      if cfg!(debug_assertions) {
        app.handle().plugin(
          tauri_plugin_log::Builder::default()
            .level(log::LevelFilter::Info)
            .build(),
        )?;
      }
      Ok(())
    })
    .invoke_handler(tauri::generate_handler![
      get_desktop_profile,
      plan_user_request,
      execute_local_action
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
