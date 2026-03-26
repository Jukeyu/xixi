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
      "Chat workspace is active".into(),
      "Desktop automation adapters are online".into(),
      "Safe command planning is now wired into the shell".into(),
    ],
  }
}

#[tauri::command]
fn plan_user_request(request: String) -> CommandPlan {
  let lowered = request.to_lowercase();
  let normalized = request.replace('：', ":");

  if lowered.contains("github") {
    return CommandPlan {
      assistant_reply:
        "我准备直接帮你打开 GitHub。这属于低风险的小动作，我会直接执行。".into(),
      risk_level: "low-risk".into(),
      can_execute_directly: true,
      steps: vec![
        step("step-open-browser", "确定目标", "将 GitHub 识别为浏览器打开动作", "done"),
        step("step-run-browser", "执行动作", "调用系统打开 GitHub 首页", "ready"),
      ],
      suggested_action: Some(LocalAction {
        kind: "open_url".into(),
        target: "https://github.com".into(),
        label: "Open GitHub".into(),
      }),
    };
  }

  if normalized.contains("天气") {
    return CommandPlan {
      assistant_reply:
        "我会先帮你打开天气查询页面，后面再把天气提醒做成真正的主动提醒模块。".into(),
      risk_level: "low-risk".into(),
      can_execute_directly: true,
      steps: vec![
        step("step-weather-plan", "理解需求", "将天气问题转成网页查询动作", "done"),
        step("step-weather-open", "执行动作", "打开天气搜索页面", "ready"),
      ],
      suggested_action: Some(LocalAction {
        kind: "open_url".into(),
        target: "https://www.bing.com/search?q=%E4%BB%8A%E5%A4%A9%E5%A4%A9%E6%B0%94".into(),
        label: "Open weather search".into(),
      }),
    };
  }

  if normalized.contains("xixi 项目")
    || normalized.contains("xixi项目")
    || normalized.contains("项目目录")
  {
    return CommandPlan {
      assistant_reply:
        "我会帮你直接打开 xixi 项目目录。这个动作只是在本地打开文件夹，风险很低。".into(),
      risk_level: "low-risk".into(),
      can_execute_directly: true,
      steps: vec![
        step("step-folder-plan", "理解需求", "识别为项目目录打开动作", "done"),
        step("step-folder-open", "执行动作", "打开 D:\\QMDownload\\xixi", "ready"),
      ],
      suggested_action: Some(LocalAction {
        kind: "open_folder".into(),
        target: r"D:\QMDownload\xixi".into(),
        label: "Open xixi project folder".into(),
      }),
    };
  }

  if normalized.contains("d 盘下载区")
    || normalized.contains("d盘下载区")
    || normalized.contains("qmdownload")
  {
    return CommandPlan {
      assistant_reply:
        "我会直接帮你打开 D 盘下载区。这个动作属于安全的小事，可以马上执行。".into(),
      risk_level: "low-risk".into(),
      can_execute_directly: true,
      steps: vec![
        step("step-qm-plan", "理解需求", "定位到 D:\\QMDownload", "done"),
        step("step-qm-open", "执行动作", "用系统文件管理器打开目标目录", "ready"),
      ],
      suggested_action: Some(LocalAction {
        kind: "open_folder".into(),
        target: r"D:\QMDownload".into(),
        label: "Open QMDownload".into(),
      }),
    };
  }

  if lowered.contains("chrome") || normalized.contains("浏览器") {
    return CommandPlan {
      assistant_reply:
        "我会优先把这个需求当成浏览器打开动作处理。当前先走安全路径，直接打开一个浏览器页面。".into(),
      risk_level: "low-risk".into(),
      can_execute_directly: true,
      steps: vec![
        step("step-browser-plan", "理解需求", "识别为浏览器打开动作", "done"),
        step("step-browser-open", "执行动作", "打开浏览器首页", "ready"),
      ],
      suggested_action: Some(LocalAction {
        kind: "open_url".into(),
        target: "https://www.google.com".into(),
        label: "Open browser".into(),
      }),
    };
  }

  CommandPlan {
    assistant_reply:
      "这句话我已经理解成一条待执行任务，但第一版还没有覆盖对应的真实执行器。我先帮你拆成动作计划，下一轮再把它接进具体的软件控制。".into(),
    risk_level: "needs-review".into(),
    can_execute_directly: false,
    steps: vec![
      step("step-intent", "识别意图", "把自然语言拆成可以执行的桌面动作", "done"),
      step("step-safety", "检查风险", "判断是否属于小事直接执行", "done"),
      step("step-adapter", "等待适配器", "当前动作需要新的本地执行器支持", "waiting"),
    ],
    suggested_action: None,
  }
}

#[tauri::command]
fn execute_local_action(action: LocalAction) -> Result<ActionExecutionResult, String> {
  match action.kind.as_str() {
    "open_folder" => open_folder(&action.target, &action.label),
    "open_url" => open_url(&action.target, &action.label),
    other => Err(format!("Unsupported action kind: {other}")),
  }
}

fn open_folder(target: &str, label: &str) -> Result<ActionExecutionResult, String> {
  Command::new("explorer")
    .arg(target)
    .spawn()
    .map_err(|error| format!("Failed to open folder: {error}"))?;

  Ok(ActionExecutionResult {
    ok: true,
    summary: format!("已经帮你打开 {label}。"),
    details: vec![target.into(), "通过 Windows 文件管理器执行".into()],
  })
}

fn open_url(target: &str, label: &str) -> Result<ActionExecutionResult, String> {
  Command::new("cmd")
    .args(["/C", "start", "", target])
    .spawn()
    .map_err(|error| format!("Failed to open url: {error}"))?;

  Ok(ActionExecutionResult {
    ok: true,
    summary: format!("已经帮你打开 {label}。"),
    details: vec![target.into(), "通过系统默认浏览器执行".into()],
  })
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
