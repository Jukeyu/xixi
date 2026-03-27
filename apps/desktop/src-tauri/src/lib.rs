use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
  env,
  fs::{self, File, OpenOptions},
  io::Write,
  path::{Path, PathBuf},
  process::{Command, Stdio},
  sync::atomic::{AtomicBool, Ordering},
  time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tauri::{
  menu::{Menu, MenuItem},
  tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
  Manager, WebviewUrl, WebviewWindowBuilder, WindowEvent,
};

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

#[derive(Serialize, Deserialize, Clone)]
struct LocalSkillDefinition {
  id: String,
  name: String,
  description: String,
  kind: String,
  target_template: String,
  label_template: Option<String>,
  risk_level: Option<String>,
  aliases: Option<Vec<String>>,
}

#[derive(Serialize, Clone)]
struct LocalSkillSummary {
  id: String,
  name: String,
  description: String,
  kind: String,
  risk_level: String,
  aliases: Vec<String>,
}

#[derive(Serialize, Deserialize)]
struct ScriptTargetPayload {
  script: String,
  input: Option<String>,
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
  action_id: String,
  duration_ms: u64,
  executed_at_ms: u64,
  recovery_tips: Vec<String>,
}

#[derive(Deserialize)]
struct ModelApiChatRequest {
  base_url: String,
  api_key: String,
  model: String,
  user_prompt: String,
  system_prompt: Option<String>,
  temperature: Option<f32>,
  max_tokens: Option<u32>,
}

#[derive(Serialize)]
struct ModelApiChatResponse {
  content: String,
  model: String,
  usage_summary: Option<String>,
  latency_ms: u64,
}

#[derive(Default)]
struct AppRuntimeState {
  quitting: AtomicBool,
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
      "Close button hides to system tray by default".into(),
      "User-defined local skills are loaded from a writable folder".into(),
    ],
  }
}

#[tauri::command]
fn list_local_skills() -> Vec<LocalSkillSummary> {
  let mut summaries = load_local_skills()
    .into_iter()
    .map(|skill| LocalSkillSummary {
      id: skill.id,
      name: skill.name,
      description: skill.description,
      kind: skill.kind.clone(),
      risk_level: skill
        .risk_level
        .unwrap_or_else(|| default_risk_for_kind(&skill.kind).into()),
      aliases: skill.aliases.unwrap_or_default(),
    })
    .collect::<Vec<_>>();

  summaries.sort_by(|a, b| a.id.cmp(&b.id));
  summaries
}

#[tauri::command]
fn get_skills_folder_path() -> String {
  skills_dir_path().to_string_lossy().into_owned()
}

#[tauri::command]
fn chat_with_model_api(request: ModelApiChatRequest) -> Result<ModelApiChatResponse, String> {
  let base_url = request.base_url.trim();
  let api_key = request.api_key.trim();
  let model = request.model.trim();
  let user_prompt = request.user_prompt.trim();

  if base_url.is_empty() {
    return Err("Model API base URL is empty.".into());
  }
  if api_key.is_empty() {
    return Err("Model API key is empty.".into());
  }
  if model.is_empty() {
    return Err("Model name is empty.".into());
  }
  if user_prompt.is_empty() {
    return Err("User prompt is empty.".into());
  }

  let endpoint = build_chat_completions_endpoint(base_url);
  let mut messages = Vec::new();

  if let Some(system_prompt) = request.system_prompt {
    let trimmed = system_prompt.trim();
    if !trimmed.is_empty() {
      messages.push(json!({
        "role": "system",
        "content": trimmed
      }));
    }
  }

  messages.push(json!({
    "role": "user",
    "content": user_prompt
  }));

  let temperature = request.temperature.unwrap_or(0.4).clamp(0.0, 2.0);
  let max_tokens = request.max_tokens.unwrap_or(512).clamp(16, 4096);

  let payload = json!({
    "model": model,
    "messages": messages,
    "temperature": temperature,
    "max_tokens": max_tokens
  });

  let client = Client::builder()
    .timeout(Duration::from_secs(45))
    .build()
    .map_err(|error| format!("Failed to build HTTP client: {error}"))?;

  let started = Instant::now();
  let response = client
    .post(&endpoint)
    .bearer_auth(api_key)
    .json(&payload)
    .send()
    .map_err(|error| format!("Failed to request model API: {error}"))?;

  let status = response.status();
  let response_text = response
    .text()
    .map_err(|error| format!("Failed to read model API response: {error}"))?;

  if !status.is_success() {
    if looks_like_html_response(&response_text) {
      let html_title = extract_html_title(&response_text).unwrap_or_else(|| "HTML upstream error".into());
      return Err(format!(
        "Model API request failed: HTTP {} ({html_title}). Upstream returned HTML instead of JSON. Endpoint used: {endpoint}. Check base URL/model/key/network. OpenAI-compatible example base URL: https://api.openai.com/v1",
        status.as_u16(),
      ));
    }

    return Err(format!(
      "Model API request failed: HTTP {} {}",
      status.as_u16(),
      truncate_error_text(&response_text, 240)
    ));
  }

  let response_json: serde_json::Value = serde_json::from_str(&response_text)
    .map_err(|error| format!("Model API response is not valid JSON: {error}"))?;
  let content = extract_model_content(&response_json)?;

  let response_model = response_json
    .get("model")
    .and_then(|value| value.as_str())
    .unwrap_or(model)
    .to_string();
  let usage_summary = response_json
    .get("usage")
    .and_then(|usage| serde_json::to_string(usage).ok());

  Ok(ModelApiChatResponse {
    content,
    model: response_model,
    usage_summary,
    latency_ms: started.elapsed().as_millis() as u64,
  })
}

#[tauri::command]
fn plan_user_request(request: String) -> CommandPlan {
  let trimmed = request.trim();
  let lowered = trimmed.to_lowercase();
  let compact = lowered.replace(' ', "");

  if let Some((skill_key, skill_input)) = extract_skill_command(trimmed) {
    return plan_skill_request(&skill_key, skill_input.as_deref());
  }

  if contains_any(
    &lowered,
    &["open skills folder", "open skill folder", "skills folder"],
  ) || contains_any(&compact, &["打开技能目录", "打开技能文件夹"])
  {
    let skills_path = skills_dir_path();
    if let Err(error) = ensure_skills_dir() {
      return unsupported_parameter_plan(
        "skills folder",
        "init failed",
        &format!("Could not initialize skills folder: {error}"),
      );
    }
    return direct_plan(
      "I can open the local skills folder right now.",
      vec![
        step(
          "plan-skills-folder-1",
          "Resolve path",
          &format!("Skills folder path: {}", skills_path.to_string_lossy()),
          "done",
        ),
        step(
          "plan-skills-folder-2",
          "Run folder open",
          "Open the writable local skills directory",
          "ready",
        ),
      ],
      LocalAction {
        kind: "open_folder".into(),
        target: skills_path.to_string_lossy().into_owned(),
        label: "xixi local skills folder".into(),
      },
    );
  }

  if contains_any(&lowered, &["open qmdownload", "qmdownload", "download folder"])
    || contains_any(&compact, &["打开qmdownload", "打开下载区", "打开d盘下载"])
  {
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

  if contains_any(&lowered, &["open xixi folder", "open xixi project", "xixi folder"])
    || contains_any(&compact, &["打开xixi目录", "打开xixi项目", "打开xixi文件夹"])
  {
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

  if contains_any(&lowered, &["open github", "github"])
    || contains_any(&compact, &["打开github", "去github", "打开代码仓库"])
  {
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

  if contains_any(&lowered, &["open weather", "weather"])
    || contains_any(&compact, &["查看天气", "打开天气", "今天天气"])
  {
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

  if contains_any(&lowered, &["open chrome", "chrome"])
    || contains_any(&compact, &["打开chrome", "打开谷歌浏览器"])
  {
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

  if contains_any(&lowered, &["open edge", "edge"])
    || contains_any(&compact, &["打开edge", "打开微软浏览器"])
  {
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

  if contains_any(&lowered, &["open firefox", "firefox", "open browser", "browser"]) {
    return direct_plan(
      "I can try to launch a browser right now.",
      vec![
        step("plan-firefox-1", "Match command", "Mapped request to Firefox launch", "done"),
        step(
          "plan-firefox-2",
          "Run app launch",
          "Try known Firefox executable locations",
          "ready",
        ),
      ],
      LocalAction {
        kind: "open_app".into(),
        target: "firefox".into(),
        label: "Mozilla Firefox".into(),
      },
    );
  }

  if contains_any(&lowered, &["open vscode", "open code", "vscode", "visual studio code"]) {
    return direct_plan(
      "I can try to launch Visual Studio Code right now.",
      vec![
        step("plan-vscode-1", "Match command", "Mapped request to VS Code launch", "done"),
        step(
          "plan-vscode-2",
          "Run app launch",
          "Try known VS Code executable locations",
          "ready",
        ),
      ],
      LocalAction {
        kind: "open_app".into(),
        target: "vscode".into(),
        label: "Visual Studio Code".into(),
      },
    );
  }

  if contains_any(&lowered, &["open terminal", "terminal", "open powershell", "powershell"]) {
    return direct_plan(
      "I can launch a terminal window right now.",
      vec![
        step("plan-terminal-1", "Match command", "Mapped request to terminal launch", "done"),
        step("plan-terminal-2", "Run app launch", "Start PowerShell terminal", "ready"),
      ],
      LocalAction {
        kind: "open_app".into(),
        target: "powershell".into(),
        label: "PowerShell".into(),
      },
    );
  }

  if contains_any(&lowered, &["open notepad", "notepad"])
    || contains_any(&compact, &["打开记事本", "打开notepad"])
  {
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

  if contains_any(&lowered, &["open explorer", "file explorer", "explorer"])
    || contains_any(&compact, &["打开资源管理器", "打开文件管理器", "打开explorer"])
  {
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

  if let Some(folder_query) = extract_after_prefix_case_insensitive(
    trimmed,
    &["open folder ", "open directory ", "open dir "],
  )
  .or_else(|| extract_after_prefix(trimmed, &["打开文件夹", "打开目录"]))
  {
    if let Some((resolved_path, display_name)) = resolve_named_folder(&folder_query) {
      return direct_plan(
        &format!("I can open {display_name} right now."),
        vec![
          step(
            "plan-folder-1",
            "Match command",
            &format!("Mapped folder alias \"{folder_query}\""),
            "done",
          ),
          step(
            "plan-folder-2",
            "Run folder open",
            &format!("Open {resolved_path} in Explorer"),
            "ready",
          ),
        ],
        LocalAction {
          kind: "open_folder".into(),
          target: resolved_path,
          label: display_name,
        },
      );
    }

    return unsupported_parameter_plan(
      "folder alias",
      &folder_query,
      "Try one of: downloads, desktop, documents, pictures, xixi folder.",
    );
  }

  if let Some(app_query) =
    extract_after_prefix_case_insensitive(trimmed, &["open app ", "launch "])
      .or_else(|| extract_after_prefix(trimmed, &["打开应用", "启动应用"]))
  {
    if let Some((app_target, app_label)) = resolve_app_alias(&app_query) {
      return direct_plan(
        &format!("I can launch {app_label} right now."),
        vec![
          step(
            "plan-app-1",
            "Match command",
            &format!("Mapped app alias \"{app_query}\""),
            "done",
          ),
          step(
            "plan-app-2",
            "Run app launch",
            &format!("Launch {app_label}"),
            "ready",
          ),
        ],
        LocalAction {
          kind: "open_app".into(),
          target: app_target.into(),
          label: app_label.into(),
        },
      );
    }

    return unsupported_parameter_plan(
      "app alias",
      &app_query,
      "Try one of: chrome, edge, firefox, vscode, terminal, powershell, cmd, taskmgr, notepad, explorer, calculator, paint, spotify, music, vlc, wmplayer.",
    );
  }

  if contains_any(&lowered, &["open music", "open music player", "play music", "open spotify"]) {
    return direct_plan(
      "I can launch a local music player right now.",
      vec![
        step("plan-music-1", "Match command", "Mapped request to music player launch", "done"),
        step(
          "plan-music-2",
          "Run app launch",
          "Try Spotify, VLC, or Windows Media Player",
          "ready",
        ),
      ],
      LocalAction {
        kind: "open_app".into(),
        target: "music".into(),
        label: "Music player".into(),
      },
    );
  }

  if let Some(raw_text) = extract_after_prefix_case_insensitive(trimmed, &["type ", "input text ", "enter text "]) {
    let text = raw_text.trim();
    if !text.is_empty() {
      if let Ok(action) = build_run_script_action(
        "safe_desktop_action.py",
        Some(format!("type:{text}")),
        "Desktop Action Safe (type)",
      ) {
        return action_plan(
          "I can type that text through local keyboard automation.",
          "high-risk",
          vec![
            step("plan-type-1", "Parse text", "Extracted text input from command", "done"),
            step("plan-type-2", "Build safe script payload", "Prepared type:* command", "done"),
            step("plan-type-3", "Run local script", "Execute keyboard typing action", "ready"),
          ],
          action,
        );
      }
    }
  }

  if let Some(raw_key) = extract_after_prefix_case_insensitive(trimmed, &["press key ", "key "]) {
    let key = raw_key.trim();
    if !key.is_empty() {
      if let Ok(action) = build_run_script_action(
        "safe_desktop_action.py",
        Some(format!("press:{key}")),
        "Desktop Action Safe (press key)",
      ) {
        return action_plan(
          "I can press that key through local keyboard automation.",
          "high-risk",
          vec![
            step("plan-press-1", "Parse key", "Extracted key input", "done"),
            step("plan-press-2", "Build safe script payload", "Prepared press:* command", "done"),
            step("plan-press-3", "Run local script", "Execute keyboard press action", "ready"),
          ],
          action,
        );
      }
    }
  }

  if let Some(raw_hotkey) = extract_after_prefix_case_insensitive(trimmed, &["hotkey ", "press hotkey "]) {
    let hotkey = raw_hotkey.trim();
    if !hotkey.is_empty() {
      if let Ok(action) = build_run_script_action(
        "safe_desktop_action.py",
        Some(format!("hotkey:{hotkey}")),
        "Desktop Action Safe (hotkey)",
      ) {
        return action_plan(
          "I can send that hotkey through local keyboard automation.",
          "high-risk",
          vec![
            step("plan-hotkey-1", "Parse hotkey", "Extracted hotkey sequence", "done"),
            step("plan-hotkey-2", "Build safe script payload", "Prepared hotkey:* command", "done"),
            step("plan-hotkey-3", "Run local script", "Execute keyboard hotkey action", "ready"),
          ],
          action,
        );
      }
    }
  }

  if let Some(raw_move) =
    extract_after_prefix_case_insensitive(trimmed, &["move mouse ", "mouse move ", "move to "])
  {
    if let Some((x, y)) = parse_coordinate_pair(&raw_move) {
      if let Ok(action) = build_run_script_action(
        "desktop_skill_ops.py",
        Some(format!("move:{x},{y}")),
        "Desktop Skill Ops (move mouse)",
      ) {
        return action_plan(
          "I can move the mouse cursor to those coordinates.",
          "high-risk",
          vec![
            step("plan-mouse-move-1", "Parse coordinates", &format!("Resolved x={x}, y={y}"), "done"),
            step("plan-mouse-move-2", "Build script payload", "Prepared move:* command", "done"),
            step("plan-mouse-move-3", "Run local script", "Execute cursor move action", "ready"),
          ],
          action,
        );
      }
    }
  }

  if let Some(raw_human_move) =
    extract_after_prefix_case_insensitive(trimmed, &["human move ", "humanized move ", "smooth move "])
  {
    if let Some((x, y)) = parse_coordinate_pair(&raw_human_move) {
      if let Ok(action) = build_run_script_action(
        "human_input_ops.py",
        Some(format!("move:{x},{y}")),
        "Human Input Ops (move)",
      ) {
        return action_plan(
          "I can move the cursor with a smoother human-like trajectory.",
          "high-risk",
          vec![
            step("plan-human-move-1", "Parse coordinates", &format!("Resolved x={x}, y={y}"), "done"),
            step("plan-human-move-2", "Build script payload", "Prepared human move command", "done"),
            step("plan-human-move-3", "Run local script", "Execute humanized cursor move", "ready"),
          ],
          action,
        );
      }
    }
  }

  if let Some(raw_human_click) =
    extract_after_prefix_case_insensitive(trimmed, &["human click ", "smooth click "])
  {
    if let Some((x, y)) = parse_coordinate_pair(&raw_human_click) {
      if let Ok(action) = build_run_script_action(
        "human_input_ops.py",
        Some(format!("click:{x},{y}")),
        "Human Input Ops (click)",
      ) {
        return action_plan(
          "I can move and click with a human-like path.",
          "high-risk",
          vec![
            step("plan-human-click-1", "Parse coordinates", &format!("Resolved x={x}, y={y}"), "done"),
            step("plan-human-click-2", "Build script payload", "Prepared human click command", "done"),
            step("plan-human-click-3", "Run local script", "Execute humanized click action", "ready"),
          ],
          action,
        );
      }
    }
  }

  if let Some(raw_human_drag) =
    extract_after_prefix_case_insensitive(trimmed, &["human drag ", "smooth drag "])
  {
    if let Some((x1, y1, x2, y2)) = parse_coordinate_quad(&raw_human_drag) {
      if let Ok(action) = build_run_script_action(
        "human_input_ops.py",
        Some(format!("drag:{x1},{y1}>{x2},{y2}")),
        "Human Input Ops (drag)",
      ) {
        return action_plan(
          "I can perform a human-like drag trajectory between two points.",
          "high-risk",
          vec![
            step(
              "plan-human-drag-1",
              "Parse coordinates",
              &format!("Resolved start=({x1},{y1}) end=({x2},{y2})"),
              "done",
            ),
            step("plan-human-drag-2", "Build script payload", "Prepared human drag command", "done"),
            step("plan-human-drag-3", "Run local script", "Execute humanized drag action", "ready"),
          ],
          action,
        );
      }
    }
  }

  if let Some(raw_human_type) =
    extract_after_prefix_case_insensitive(trimmed, &["human type ", "smooth type "])
  {
    let text = raw_human_type.trim();
    if !text.is_empty() {
      if let Ok(action) = build_run_script_action(
        "human_input_ops.py",
        Some(format!("type:{text}")),
        "Human Input Ops (type)",
      ) {
        return action_plan(
          "I can type text with human-like randomized cadence.",
          "high-risk",
          vec![
            step("plan-human-type-1", "Parse text", "Extracted typing content", "done"),
            step("plan-human-type-2", "Build script payload", "Prepared human type command", "done"),
            step("plan-human-type-3", "Run local script", "Execute humanized typing", "ready"),
          ],
          action,
        );
      }
    }
  }

  if let Some(raw_scroll_up) = extract_after_prefix_case_insensitive(trimmed, &["scroll up ", "scrollup "]) {
    let amount = parse_first_int(&raw_scroll_up).unwrap_or(350).abs();
    if let Ok(action) = build_run_script_action(
      "desktop_skill_ops.py",
      Some(format!("scroll:{amount}")),
      "Desktop Skill Ops (scroll up)",
    ) {
      return action_plan(
        "I can scroll the active window upward.",
        "high-risk",
        vec![
          step("plan-scroll-up-1", "Parse amount", &format!("Scroll amount={amount}"), "done"),
          step("plan-scroll-up-2", "Build script payload", "Prepared scroll:* command", "done"),
          step("plan-scroll-up-3", "Run local script", "Execute scroll action", "ready"),
        ],
        action,
      );
    }
  }

  if let Some(raw_scroll_down) = extract_after_prefix_case_insensitive(trimmed, &["scroll down ", "scrolldown "]) {
    let amount = parse_first_int(&raw_scroll_down).unwrap_or(350).abs();
    if let Ok(action) = build_run_script_action(
      "desktop_skill_ops.py",
      Some(format!("scroll:-{amount}")),
      "Desktop Skill Ops (scroll down)",
    ) {
      return action_plan(
        "I can scroll the active window downward.",
        "high-risk",
        vec![
          step("plan-scroll-down-1", "Parse amount", &format!("Scroll amount={amount}"), "done"),
          step("plan-scroll-down-2", "Build script payload", "Prepared scroll:* command", "done"),
          step("plan-scroll-down-3", "Run local script", "Execute scroll action", "ready"),
        ],
        action,
      );
    }
  }

  if lowered == "double click" || lowered == "double-click" || lowered == "mouse double click" {
    if let Ok(action) = build_run_script_action(
      "desktop_skill_ops.py",
      Some("doubleclick".to_string()),
      "Desktop Skill Ops (double click)",
    ) {
      return action_plan(
        "I can execute a double-click on the current cursor position.",
        "high-risk",
        vec![
          step("plan-doubleclick-1", "Match command", "Detected double-click intent", "done"),
          step("plan-doubleclick-2", "Build script payload", "Prepared doubleclick command", "done"),
          step("plan-doubleclick-3", "Run local script", "Execute double-click action", "ready"),
        ],
        action,
      );
    }
  }

  if lowered == "right click" || lowered == "right-click" || lowered == "mouse right click" {
    if let Ok(action) = build_run_script_action(
      "desktop_skill_ops.py",
      Some("rightclick".to_string()),
      "Desktop Skill Ops (right click)",
    ) {
      return action_plan(
        "I can execute a right-click on the current cursor position.",
        "high-risk",
        vec![
          step("plan-rightclick-1", "Match command", "Detected right-click intent", "done"),
          step("plan-rightclick-2", "Build script payload", "Prepared rightclick command", "done"),
          step("plan-rightclick-3", "Run local script", "Execute right-click action", "ready"),
        ],
        action,
      );
    }
  }

  if lowered == "click" || lowered == "left click" || lowered == "mouse click" || lowered == "click mouse" {
    if let Ok(action) = build_run_script_action(
      "desktop_skill_ops.py",
      Some("click".to_string()),
      "Desktop Skill Ops (click)",
    ) {
      return action_plan(
        "I can execute a left-click on the current cursor position.",
        "high-risk",
        vec![
          step("plan-click-1", "Match command", "Detected left-click intent", "done"),
          step("plan-click-2", "Build script payload", "Prepared click command", "done"),
          step("plan-click-3", "Run local script", "Execute click action", "ready"),
        ],
        action,
      );
    }
  }

  if lowered == "scroll up" {
    if let Ok(action) = build_run_script_action(
      "desktop_skill_ops.py",
      Some("scroll:350".to_string()),
      "Desktop Skill Ops (scroll up)",
    ) {
      return action_plan(
        "I can scroll the active window upward.",
        "high-risk",
        vec![
          step("plan-scroll-up-default-1", "Match command", "Detected default scroll-up command", "done"),
          step("plan-scroll-up-default-2", "Build script payload", "Prepared scroll:350 command", "done"),
          step("plan-scroll-up-default-3", "Run local script", "Execute scroll action", "ready"),
        ],
        action,
      );
    }
  }

  if lowered == "scroll down" {
    if let Ok(action) = build_run_script_action(
      "desktop_skill_ops.py",
      Some("scroll:-350".to_string()),
      "Desktop Skill Ops (scroll down)",
    ) {
      return action_plan(
        "I can scroll the active window downward.",
        "high-risk",
        vec![
          step("plan-scroll-down-default-1", "Match command", "Detected default scroll-down command", "done"),
          step("plan-scroll-down-default-2", "Build script payload", "Prepared scroll:-350 command", "done"),
          step("plan-scroll-down-default-3", "Run local script", "Execute scroll action", "ready"),
        ],
        action,
      );
    }
  }

  if lowered == "screen intent"
    || lowered == "watch intent"
    || lowered == "watch screen intent"
    || lowered == "intent watch"
  {
    if let Ok(action) = build_run_script_action(
      "screen_intent_watch.py",
      Some("goal=desktop-workflow duration=18 interval=1.2 samples=8".to_string()),
      "Screen Intent Watch",
    ) {
      return action_plan(
        "I can run a real screen-intent observer now and summarize likely user intent.",
        "medium-risk",
        vec![
          step(
            "plan-intent-1",
            "Match intent command",
            "Using default intent-observation profile",
            "done",
          ),
          step(
            "plan-intent-2",
            "Build script payload",
            "Prepared screen_intent_watch args",
            "done",
          ),
          step(
            "plan-intent-3",
            "Run local script",
            "Observe active window + OCR signals",
            "ready",
          ),
        ],
        action,
      );
    }
  }

  if let Some(raw_intent) = extract_after_prefix_case_insensitive(
    trimmed,
    &["screen intent ", "watch intent ", "watch screen intent ", "intent watch "],
  ) {
    let goal = raw_intent
      .split_whitespace()
      .collect::<Vec<_>>()
      .join("_");
    let input = if goal.is_empty() {
      "goal=desktop-workflow duration=18 interval=1.2 samples=8".to_string()
    } else {
      format!("goal={goal} duration=18 interval=1.2 samples=8")
    };
    if let Ok(action) = build_run_script_action("screen_intent_watch.py", Some(input), "Screen Intent Watch")
    {
      return action_plan(
        "I can run a real screen-intent observer now and summarize likely user intent.",
        "medium-risk",
        vec![
          step(
            "plan-intent-1",
            "Parse intent hint",
            "Resolved screen intent observation hint",
            "done",
          ),
          step(
            "plan-intent-2",
            "Build script payload",
            "Prepared screen_intent_watch args",
            "done",
          ),
          step(
            "plan-intent-3",
            "Run local script",
            "Observe active window + OCR signals",
            "ready",
          ),
        ],
        action,
      );
    }
  }

  if lowered == "watch screen" || lowered == "screen watch" {
    if let Ok(action) = build_run_script_action(
      "screen_watch_ocr.py",
      Some("keyword=stock duration=20 interval=1 max_hits=2".to_string()),
      "Screen Watch OCR",
    ) {
      return action_plan(
        "I can start a real screen-watch OCR task now.",
        "medium-risk",
        vec![
          step("plan-watch-1", "Parse watch target", "Using default screen watch keyword", "done"),
          step("plan-watch-2", "Build OCR script payload", "Prepared screen_watch_ocr args", "done"),
          step("plan-watch-3", "Run local script", "Execute OCR watch loop", "ready"),
        ],
        action,
      );
    }
  }

  if let Some(raw_watch) = extract_after_prefix_case_insensitive(trimmed, &["watch screen ", "screen watch "]) {
    let keyword = raw_watch.trim();
    let keyword_lower = keyword.to_lowercase();
    if keyword_lower.starts_with("behavior") {
      let goal = keyword["behavior".len()..]
        .trim()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("_");
      let input = if goal.is_empty() {
        "goal=workflow duration=16 interval=1.0 samples=10".to_string()
      } else {
        format!("goal={goal} duration=16 interval=1.0 samples=10")
      };
      if let Ok(action) =
        build_run_script_action("screen_behavior_watch.py", Some(input), "Screen Behavior Watch")
      {
        return action_plan(
          "I can run a real screen+mouse behavior observer now.",
          "medium-risk",
          vec![
            step(
              "plan-behavior-1",
              "Parse behavior hint",
              "Resolved behavior observation hint",
              "done",
            ),
            step(
              "plan-behavior-2",
              "Build script payload",
              "Prepared screen_behavior_watch args",
              "done",
            ),
            step(
              "plan-behavior-3",
              "Run local script",
              "Observe screen dynamics + cursor trajectory",
              "ready",
            ),
          ],
          action,
        );
      }
    }
    let input = if keyword.is_empty() {
      "keyword=stock duration=20 interval=1 max_hits=2".to_string()
    } else {
      format!("keyword={keyword} duration=20 interval=1 max_hits=2")
    };
    if let Ok(action) = build_run_script_action("screen_watch_ocr.py", Some(input), "Screen Watch OCR") {
      return action_plan(
        "I can start a real screen-watch OCR task now.",
        "medium-risk",
        vec![
          step("plan-watch-1", "Parse watch target", "Resolved OCR watch keyword", "done"),
          step("plan-watch-2", "Build OCR script payload", "Prepared screen_watch_ocr args", "done"),
          step("plan-watch-3", "Run local script", "Execute OCR watch loop", "ready"),
        ],
        action,
      );
    }
  }

  if lowered == "watch screen behavior"
    || lowered == "screen behavior watch"
    || lowered == "watch desktop behavior"
    || lowered == "watch mouse behavior"
  {
    if let Ok(action) = build_run_script_action(
      "screen_behavior_watch.py",
      Some("goal=workflow duration=16 interval=1.0 samples=10".to_string()),
      "Screen Behavior Watch",
    ) {
      return action_plan(
        "I can run a real screen+mouse behavior observer now.",
        "medium-risk",
        vec![
          step(
            "plan-behavior-1",
            "Match behavior command",
            "Using default behavior-observation profile",
            "done",
          ),
          step(
            "plan-behavior-2",
            "Build script payload",
            "Prepared screen_behavior_watch args",
            "done",
          ),
          step(
            "plan-behavior-3",
            "Run local script",
            "Observe screen dynamics + cursor trajectory",
            "ready",
          ),
        ],
        action,
      );
    }
  }

  if let Some(raw_behavior) = extract_after_prefix_case_insensitive(
    trimmed,
    &[
      "watch screen behavior ",
      "screen behavior watch ",
      "watch desktop behavior ",
      "watch mouse behavior ",
    ],
  ) {
    let goal = raw_behavior
      .split_whitespace()
      .collect::<Vec<_>>()
      .join("_");
    let input = if goal.is_empty() {
      "goal=workflow duration=16 interval=1.0 samples=10".to_string()
    } else {
      format!("goal={goal} duration=16 interval=1.0 samples=10")
    };
    if let Ok(action) =
      build_run_script_action("screen_behavior_watch.py", Some(input), "Screen Behavior Watch")
    {
      return action_plan(
        "I can run a real screen+mouse behavior observer now.",
        "medium-risk",
        vec![
          step(
            "plan-behavior-1",
            "Parse behavior hint",
            "Resolved behavior observation hint",
            "done",
          ),
          step(
            "plan-behavior-2",
            "Build script payload",
            "Prepared screen_behavior_watch args",
            "done",
          ),
          step(
            "plan-behavior-3",
            "Run local script",
            "Observe screen dynamics + cursor trajectory",
            "ready",
          ),
        ],
        action,
      );
    }
  }

  if lowered == "latest screen behavior"
    || lowered == "screen behavior report"
    || lowered == "latest behavior report"
  {
    return direct_plan(
      "I can read the latest screen-behavior observation report now.",
      vec![
        step(
          "plan-behavior-report-1",
          "Resolve command",
          "Matched latest behavior report query",
          "done",
        ),
        step(
          "plan-behavior-report-2",
          "Read latest run log",
          "Load most recent screen_behavior_watch log",
          "ready",
        ),
      ],
      LocalAction {
        kind: "read_behavior_report".into(),
        target: "screen_behavior_watch".into(),
        label: "Latest Screen Behavior Report".into(),
      },
    );
  }

  if lowered == "latest screen intent"
    || lowered == "screen intent report"
    || lowered == "intent report"
    || lowered == "what is on screen"
  {
    return direct_plan(
      "I can read the latest screen-intent observation report now.",
      vec![
        step(
          "plan-intent-report-1",
          "Resolve command",
          "Matched latest intent report query",
          "done",
        ),
        step(
          "plan-intent-report-2",
          "Read latest run log",
          "Load most recent screen_intent_watch log",
          "ready",
        ),
      ],
      LocalAction {
        kind: "read_intent_report".into(),
        target: "screen_intent_watch".into(),
        label: "Latest Screen Intent Report".into(),
      },
    );
  }

  if let Some(site_target) = extract_after_prefix_case_insensitive(
    trimmed,
    &["open site ", "open website ", "open url ", "visit "],
  )
  .or_else(|| extract_after_prefix(trimmed, &["打开网站", "打开网页", "访问"]))
  {
    let (url, label) = normalize_site_target(&site_target);
    return direct_plan(
      "I can open that site right now.",
      vec![
        step("plan-site-1", "Normalize target", &format!("Resolved URL: {url}"), "done"),
        step(
          "plan-site-2",
          "Run browser open",
          "Open URL in default browser",
          "ready",
        ),
      ],
      LocalAction {
        kind: "open_url".into(),
        target: url,
        label,
      },
    );
  }

  if let Some(search_query) = extract_after_prefix_case_insensitive(trimmed, &["search web ", "search "])
    .or_else(|| extract_after_prefix(trimmed, &["搜索", "搜一下"]))
  {
    let query = search_query.trim();
    if !query.is_empty() {
      let url = build_search_url(query);
      return direct_plan(
        "I can run a real web search in your browser right now.",
        vec![
          step("plan-search-1", "Normalize query", &format!("Search query: {query}"), "done"),
          step(
            "plan-search-2",
            "Run browser open",
            "Open search URL in default browser",
            "ready",
          ),
        ],
        LocalAction {
          kind: "search_web".into(),
          target: url,
          label: format!("web search: {query}"),
        },
      );
    }
  }

  if let Some(maybe_site) = extract_after_prefix_case_insensitive(trimmed, &["open "]) {
    let candidate = maybe_site.trim();
    if candidate.contains('.') {
      let (url, label) = normalize_site_target(candidate);
      return direct_plan(
        "I recognized that as a site target and can open it now.",
        vec![
          step("plan-open-1", "Infer intent", "Detected site-like target after open", "done"),
          step("plan-open-2", "Run browser open", "Open inferred URL", "ready"),
        ],
        LocalAction {
          kind: "open_url".into(),
          target: url,
          label,
        },
      );
    }
  }

  unsupported_plan()
}

#[tauri::command]
fn execute_local_action(action: LocalAction) -> ActionExecutionResult {
  let action_id = format!("act-{}", now_unix_ms());
  let started_at = now_unix_ms();
  let timer = Instant::now();

  let execution = match action.kind.as_str() {
    "open_folder" => open_folder(&action.target, &action.label),
    "open_url" => open_url(&action.target, &action.label),
    "search_web" => open_url(&action.target, &action.label),
    "open_app" => open_app(&action.target, &action.label),
    "run_script" => run_script(&action.target, &action.label),
    "read_intent_report" => read_latest_screen_intent_report(&action.target, &action.label),
    "read_behavior_report" => read_latest_screen_behavior_report(&action.target, &action.label),
    other => Err(format!("Unsupported action kind: {other}")),
  };

  let (ok, summary, mut details, recovery_tips) = match execution {
    Ok((summary, details)) => (true, summary, details, Vec::new()),
    Err(error) => (
      false,
      format!("Failed to run {}.", action.label),
      vec![error],
      recovery_tips_for_action(&action),
    ),
  };

  details.push(format!("kind={}", action.kind));
  details.push(format!("target={}", action.target));

  let result = ActionExecutionResult {
    ok,
    summary,
    details,
    action_id,
    duration_ms: timer.elapsed().as_millis() as u64,
    executed_at_ms: started_at as u64,
    recovery_tips,
  };

  if let Err(error) = append_action_log(&action, &result) {
    eprintln!("failed to append action log: {error}");
  }

  result
}

#[tauri::command]
fn quit_application(app: tauri::AppHandle, state: tauri::State<AppRuntimeState>) {
  state.quitting.store(true, Ordering::Relaxed);
  app.exit(0);
}

#[tauri::command]
fn minimize_to_pet(app: tauri::AppHandle) {
  hide_main_window(&app);
}

#[tauri::command]
fn restore_main_from_pet(app: tauri::AppHandle) {
  show_main_window(&app);
}

fn action_plan(reply: &str, risk_level: &str, steps: Vec<ActionItem>, action: LocalAction) -> CommandPlan {
  CommandPlan {
    assistant_reply: reply.into(),
    risk_level: risk_level.into(),
    can_execute_directly: true,
    steps,
    suggested_action: Some(action),
  }
}

fn direct_plan(reply: &str, steps: Vec<ActionItem>, action: LocalAction) -> CommandPlan {
  action_plan(reply, "low-risk", steps, action)
}

fn build_run_script_action(script: &str, input: Option<String>, label: &str) -> Result<LocalAction, String> {
  let payload = ScriptTargetPayload {
    script: script.into(),
    input,
  };
  let target = serde_json::to_string(&payload)
    .map_err(|error| format!("Failed to build script action payload: {error}"))?;
  Ok(LocalAction {
    kind: "run_script".into(),
    target,
    label: label.into(),
  })
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

fn plan_skill_request(skill_key: &str, skill_input: Option<&str>) -> CommandPlan {
  let skills = load_local_skills();
  let normalized_key = normalize_alias(skill_key);

  if skills.is_empty() {
    return unsupported_parameter_plan(
      "skill id",
      skill_key,
      "No local skills found yet. Open skills folder and add a JSON skill file.",
    );
  }

  let Some(skill) = resolve_local_skill(&skills, &normalized_key) else {
    let suggestions = skills
      .iter()
      .take(5)
      .map(|item| item.id.as_str())
      .collect::<Vec<_>>()
      .join(", ");
    return unsupported_parameter_plan(
      "skill id",
      skill_key,
      &format!("Known skills: {suggestions}"),
    );
  };

  let input = skill_input.unwrap_or("").trim();
  let action = match render_skill_action(skill, input) {
    Ok(action) => action,
    Err(message) => {
      return unsupported_parameter_plan("skill input", input, &message);
    }
  };

  CommandPlan {
    assistant_reply: format!(
      "I can run skill \"{}\"{}.",
      skill.name,
      if input.is_empty() {
        String::new()
      } else {
        format!(" with input \"{input}\"")
      }
    ),
    risk_level: skill
      .risk_level
      .clone()
      .unwrap_or_else(|| default_risk_for_kind(&skill.kind).into()),
    can_execute_directly: true,
    steps: vec![
      step(
        "plan-skill-1",
        "Match skill",
        &format!("Resolved skill {} ({})", skill.name, skill.id),
        "done",
      ),
      step(
        "plan-skill-2",
        "Render action",
        &format!("kind={} target={}", action.kind, action.target),
        "done",
      ),
      step(
        "plan-skill-3",
        "Run action",
        "Execute skill-generated local action",
        "ready",
      ),
    ],
    suggested_action: Some(action),
  }
}

fn extract_skill_command(request: &str) -> Option<(String, Option<String>)> {
  let lowered = request.to_lowercase();
  let prefixes = [
    "run skill ",
    "use skill ",
    "execute skill ",
    "执行技能",
    "运行技能",
    "技能 ",
  ];

  let remainder = prefixes.iter().find_map(|prefix| {
    if lowered.starts_with(prefix) {
      Some(request[prefix.len()..].trim().to_string())
    } else {
      None
    }
  })?;

  if remainder.is_empty() {
    return None;
  }

  let mut parts = remainder.splitn(2, char::is_whitespace);
  let skill_key = parts.next()?.trim().to_string();
  if skill_key.is_empty() {
    return None;
  }
  let input = parts.next().map(|value| value.trim().to_string()).filter(|v| !v.is_empty());
  Some((skill_key, input))
}

fn resolve_local_skill<'a>(
  skills: &'a [LocalSkillDefinition],
  normalized_key: &str,
) -> Option<&'a LocalSkillDefinition> {
  skills.iter().find(|skill| {
    normalize_alias(&skill.id) == normalized_key
      || skill
        .aliases
        .as_ref()
        .is_some_and(|aliases| aliases.iter().any(|alias| normalize_alias(alias) == normalized_key))
  })
}

fn render_skill_action(skill: &LocalSkillDefinition, input: &str) -> Result<LocalAction, String> {
  let requires_input = skill.target_template.contains("{{input}}")
    || skill
      .label_template
      .as_ref()
      .is_some_and(|template| template.contains("{{input}}"));

  if requires_input && input.is_empty() {
    return Err(format!(
      "Skill {} requires input. Use: run skill {} <your input>",
      skill.name, skill.id
    ));
  }

  let rendered_target = skill.target_template.replace("{{input}}", input);
  let rendered_label = skill
    .label_template
    .as_ref()
    .unwrap_or(&skill.name)
    .replace("{{input}}", input);

  let target = match skill.kind.as_str() {
    "open_url" => normalize_site_target(&rendered_target).0,
    "search_web" => {
      if rendered_target.starts_with("http://") || rendered_target.starts_with("https://") {
        rendered_target
      } else {
        build_search_url(&rendered_target)
      }
    }
    "open_folder" | "open_app" => rendered_target,
    "run_script" => {
      let payload = ScriptTargetPayload {
        script: rendered_target,
        input: if input.is_empty() {
          None
        } else {
          Some(input.to_string())
        },
      };
      serde_json::to_string(&payload)
        .map_err(|error| format!("Failed to build script payload: {error}"))?
    }
    other => return Err(format!("Unsupported skill kind: {other}")),
  };

  Ok(LocalAction {
    kind: skill.kind.clone(),
    target,
    label: rendered_label,
  })
}

fn skills_dir_path() -> PathBuf {
  if let Ok(local_app_data) = env::var("LOCALAPPDATA") {
    return Path::new(&local_app_data).join("xixi").join("skills");
  }
  env::temp_dir().join("xixi").join("skills")
}

fn ensure_skills_dir() -> Result<(), String> {
  let dir = skills_dir_path();
  fs::create_dir_all(&dir).map_err(|error| format!("Failed to create skills folder: {error}"))?;
  let scripts_dir = skills_scripts_dir();
  fs::create_dir_all(&scripts_dir)
    .map_err(|error| format!("Failed to create skills scripts folder: {error}"))?;
  let runs_dir = skills_runs_dir();
  fs::create_dir_all(&runs_dir)
    .map_err(|error| format!("Failed to create skills runs folder: {error}"))?;

  for skill in default_local_skills() {
    let path = dir.join(format!("{}.json", skill.id));
    if path.exists() {
      continue;
    }
    let content = serde_json::to_string_pretty(&skill)
      .map_err(|error| format!("Failed to serialize default skill: {error}"))?;
    fs::write(&path, content).map_err(|error| format!("Failed to write default skill: {error}"))?;
  }

  ensure_default_script(
    &scripts_dir.join("screen_watch_ocr.py"),
    r#"import datetime
import time
import sys

def parse_options(raw: str):
    defaults = {
        "keyword": "stock",
        "interval": "1.0",
        "duration": "20",
        "max_hits": "3",
        "region": "",
    }
    text = (raw or "").strip()
    if "=" not in text:
        if text:
            defaults["keyword"] = text
        return defaults

    parts = [p.strip() for p in text.split() if p.strip()]
    for part in parts:
        if "=" not in part:
            continue
        k, v = part.split("=", 1)
        k = k.strip().lower()
        v = v.strip()
        if k in defaults and v:
            defaults[k] = v
    return defaults

def parse_region(region_text: str):
    if not region_text:
        return None
    values = [v.strip() for v in region_text.split(",")]
    if len(values) != 4:
        return None
    try:
        left, top, width, height = [int(v) for v in values]
        if width <= 0 or height <= 0:
            return None
        return {"left": left, "top": top, "width": width, "height": height}
    except Exception:
        return None

def log(msg: str):
    now = datetime.datetime.now().isoformat(timespec="seconds")
    print(f"[{now}] {msg}", flush=True)

def main():
    raw = sys.argv[1] if len(sys.argv) > 1 else ""
    opts = parse_options(raw)
    keyword = opts["keyword"].lower()

    try:
        interval = max(0.2, float(opts["interval"]))
    except Exception:
        interval = 1.0

    try:
        duration = max(3.0, float(opts["duration"]))
    except Exception:
        duration = 20.0

    try:
        max_hits = max(1, int(opts["max_hits"]))
    except Exception:
        max_hits = 3

    region = parse_region(opts["region"])
    log(f"screen_watch_ocr start keyword={keyword} interval={interval}s duration={duration}s max_hits={max_hits} region={region or 'full-screen'}")

    try:
        import mss
        from PIL import Image
        import pytesseract
    except Exception as e:
        log("missing dependency. install with:")
        log("pip install mss pillow pytesseract")
        log(f"import error: {e}")
        raise SystemExit(1)

    hits = 0
    scans = 0
    started = time.time()
    with mss.mss() as sct:
        monitor = region or sct.monitors[1]
        while time.time() - started <= duration:
            shot = sct.grab(monitor)
            image = Image.frombytes("RGB", shot.size, shot.rgb)
            text = pytesseract.image_to_string(image)
            scans += 1
            if keyword in text.lower():
                hits += 1
                preview = " ".join(text.split())[:180]
                log(f"HIT {hits}/{max_hits}: keyword found, preview={preview}")
                if hits >= max_hits:
                    break
            time.sleep(interval)

    log(f"done scans={scans} hits={hits}")

if __name__ == '__main__':
    main()
"#,
  )?;

  ensure_default_script(
    &scripts_dir.join("screen_intent_watch.py"),
    r#"import ctypes
import ctypes.wintypes as wintypes
import datetime
import json
import os
import sys
import time
from collections import Counter

PROCESS_QUERY_LIMITED_INFORMATION = 0x1000

INTENT_RULES = {
    "coding": [
        "vscode", "visual studio code", "terminal", "powershell", "cmd", "github", "gitlab",
        "stack overflow", "traceback", "exception", "cargo", "npm", "python", "rust", "commit", "pull request",
    ],
    "writing": [
        "notion", "word", "docs", "document", "report", "proposal", "draft", "markdown", "slides",
    ],
    "research": [
        "search", "bing", "google", "wiki", "article", "documentation", "tutorial", "readme",
    ],
    "communication": [
        "slack", "discord", "telegram", "mail", "gmail", "outlook", "message", "chat", "teams",
    ],
    "meeting": [
        "zoom", "google meet", "meet", "webex", "meeting", "calendar invite",
    ],
    "trading": [
        "tradingview", "stock", "crypto", "chart", "portfolio", "order", "buy", "sell", "broker",
    ],
    "media": [
        "spotify", "youtube", "music", "movie", "video", "netflix",
    ],
    "file_management": [
        "explorer", "folder", "file", "downloads", "desktop", "documents",
    ],
}

INTENT_TO_SUGGESTIONS = {
    "coding": [
        "open app vscode",
        "search web <error keyword>",
        "run skill desktop_skill_ops hotkey:ctrl,s",
    ],
    "writing": [
        "open app notepad",
        "type <draft text>",
    ],
    "research": [
        "search web <topic>",
        "watch screen <keyword>",
    ],
    "communication": [
        "open app edge",
        "watch screen <message keyword>",
    ],
    "meeting": [
        "open site meet.google.com",
        "open app edge",
    ],
    "trading": [
        "run skill open_tradingview",
        "watch screen stock",
    ],
    "media": [
        "open app spotify",
        "open music player",
    ],
    "file_management": [
        "open folder downloads",
        "open xixi folder",
    ],
}

def log(msg: str):
    now = datetime.datetime.now().isoformat(timespec="seconds")
    print(f"[{now}] {msg}", flush=True)

def parse_options(raw: str):
    defaults = {
        "goal": "desktop-workflow",
        "duration": "18",
        "interval": "1.2",
        "samples": "8",
        "max_chars": "1600",
        "ocr": "1",
        "region": "",
    }
    text = (raw or "").strip()
    if not text:
        return defaults

    parts = [p.strip() for p in text.split() if p.strip()]
    if parts and "=" not in parts[0]:
        defaults["goal"] = parts[0]

    for part in parts:
        if "=" not in part:
            continue
        key, value = part.split("=", 1)
        key = key.strip().lower()
        value = value.strip()
        if key in defaults and value:
            defaults[key] = value
    return defaults

def parse_region(region_text: str):
    if not region_text:
        return None
    values = [v.strip() for v in region_text.split(",")]
    if len(values) != 4:
        return None
    try:
        left, top, width, height = [int(v) for v in values]
        if width <= 0 or height <= 0:
            return None
        return {"left": left, "top": top, "width": width, "height": height}
    except Exception:
        return None

def normalize_text(text: str, max_chars: int):
    compact = " ".join((text or "").split())
    return compact[:max_chars]

def parse_float(value: str, default: float, min_v: float, max_v: float):
    try:
        parsed = float(value)
    except Exception:
        return default
    return max(min_v, min(max_v, parsed))

def parse_int(value: str, default: int, min_v: int, max_v: int):
    try:
        parsed = int(value)
    except Exception:
        return default
    return max(min_v, min(max_v, parsed))

def parse_bool(value: str):
    return str(value).strip().lower() in {"1", "true", "yes", "on"}

def resolve_process_name(pid: int):
    if not pid:
        return ""
    try:
        kernel32 = ctypes.windll.kernel32
        kernel32.OpenProcess.argtypes = [wintypes.DWORD, wintypes.BOOL, wintypes.DWORD]
        kernel32.OpenProcess.restype = wintypes.HANDLE
        kernel32.QueryFullProcessImageNameW.argtypes = [
            wintypes.HANDLE,
            wintypes.DWORD,
            wintypes.LPWSTR,
            ctypes.POINTER(wintypes.DWORD),
        ]
        kernel32.QueryFullProcessImageNameW.restype = wintypes.BOOL
        kernel32.CloseHandle.argtypes = [wintypes.HANDLE]
        kernel32.CloseHandle.restype = wintypes.BOOL

        handle = kernel32.OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, False, pid)
        if not handle:
            return ""
        try:
            size = wintypes.DWORD(1024)
            buffer = ctypes.create_unicode_buffer(size.value)
            ok = kernel32.QueryFullProcessImageNameW(handle, 0, buffer, ctypes.byref(size))
            if not ok:
                return ""
            return os.path.basename(buffer.value).lower()
        finally:
            kernel32.CloseHandle(handle)
    except Exception:
        return ""

def foreground_window_info():
    try:
        user32 = ctypes.windll.user32
        hwnd = user32.GetForegroundWindow()
        if not hwnd:
            return {"title": "", "process": "", "pid": 0}

        length = user32.GetWindowTextLengthW(hwnd)
        length = max(0, min(length, 2048))
        buffer = ctypes.create_unicode_buffer(length + 1)
        user32.GetWindowTextW(hwnd, buffer, len(buffer))
        title = buffer.value.strip()

        pid = wintypes.DWORD(0)
        user32.GetWindowThreadProcessId(hwnd, ctypes.byref(pid))
        process_name = resolve_process_name(pid.value)
        return {"title": title, "process": process_name, "pid": int(pid.value)}
    except Exception:
        return {"title": "", "process": "", "pid": 0}

def capture_ocr_text(region, max_chars: int):
    try:
        import mss
        from PIL import Image
        import pytesseract
    except Exception as error:
        return "", str(error)

    with mss.mss() as sct:
        monitor = region or sct.monitors[1]
        shot = sct.grab(monitor)
        image = Image.frombytes("RGB", shot.size, shot.rgb)
        text = pytesseract.image_to_string(image, config="--psm 6")
        return normalize_text(text, max_chars), ""

def infer_intents(window_title: str, process_name: str, ocr_text: str, goal_hint: str):
    combined = " ".join([window_title, process_name, ocr_text]).lower()
    goal_hint = (goal_hint or "").replace("_", " ").strip().lower()
    goal_tokens = [token for token in goal_hint.split() if len(token) >= 3]
    intents = []

    for label, keywords in INTENT_RULES.items():
        hits = [keyword for keyword in keywords if keyword in combined]
        if not hits:
            continue
        confidence = min(0.95, 0.32 + 0.1 * len(hits))
        if goal_tokens and any(token in combined for token in goal_tokens):
            confidence = min(0.97, confidence + 0.06)
        intents.append(
            {
                "label": label,
                "confidence": round(confidence, 2),
                "evidence": hits[:6],
            }
        )

    intents.sort(key=lambda item: item["confidence"], reverse=True)
    if intents:
        return intents

    return [
        {
            "label": "unknown",
            "confidence": 0.22,
            "evidence": ["insufficient_keywords"],
        }
    ]

def build_suggestions(dominant_intent: str):
    return INTENT_TO_SUGGESTIONS.get(
        dominant_intent,
        [
            "watch screen <keyword>",
            "search web <goal>",
        ],
    )

def main():
    raw = sys.argv[1] if len(sys.argv) > 1 else ""
    opts = parse_options(raw)
    goal = opts["goal"]
    duration = parse_float(opts["duration"], default=18.0, min_v=4.0, max_v=120.0)
    interval = parse_float(opts["interval"], default=1.2, min_v=0.4, max_v=10.0)
    samples = parse_int(opts["samples"], default=8, min_v=1, max_v=80)
    max_chars = parse_int(opts["max_chars"], default=1600, min_v=200, max_v=6000)
    use_ocr = parse_bool(opts["ocr"])
    region = parse_region(opts["region"])

    max_by_duration = max(1, int(duration / interval))
    total_samples = min(samples, max_by_duration)

    log(
        f"screen_intent_watch start goal={goal} duration={duration}s interval={interval}s "
        f"samples={total_samples} ocr={use_ocr} region={region or 'full-screen'}"
    )

    if use_ocr:
        try:
            import mss  # noqa: F401
            import PIL  # noqa: F401
            import pytesseract  # noqa: F401
        except Exception as error:
            log("OCR dependency missing, fallback to active-window-only intent mode.")
            log("install OCR dependencies with: pip install mss pillow pytesseract")
            log(f"import error: {error}")
            use_ocr = False

    intent_counter = Counter()
    window_counter = Counter()
    process_counter = Counter()
    sample_rows = []
    ocr_errors = []

    started_at = time.time()
    for index in range(total_samples):
        window_info = foreground_window_info()
        window_title = window_info.get("title", "")
        process_name = window_info.get("process", "")
        pid = window_info.get("pid", 0)

        ocr_text = ""
        if use_ocr:
            ocr_text, ocr_error = capture_ocr_text(region, max_chars=max_chars)
            if ocr_error:
                ocr_errors.append(ocr_error)

        intents = infer_intents(window_title, process_name, ocr_text, goal)
        top_intent = intents[0]["label"]
        intent_counter[top_intent] += 1
        if window_title:
            window_counter[window_title] += 1
        if process_name:
            process_counter[process_name] += 1

        sample_rows.append(
            {
                "index": index + 1,
                "timestamp": datetime.datetime.now().isoformat(timespec="seconds"),
                "window_title": window_title,
                "process": process_name,
                "pid": pid,
                "top_intent": top_intent,
                "intent_candidates": intents[:3],
                "ocr_preview": ocr_text[:200],
            }
        )

        confidence = intents[0]["confidence"]
        log(
            f"sample {index + 1}/{total_samples} process={process_name or '-'} "
            f"title={window_title[:60] or '-'} top_intent={top_intent} confidence={confidence}"
        )

        if index < total_samples - 1:
            elapsed = time.time() - started_at
            if elapsed + interval > duration:
                break
            time.sleep(interval)

    dominant_intent = intent_counter.most_common(1)[0][0] if intent_counter else "unknown"
    dominant_window = window_counter.most_common(1)[0][0] if window_counter else ""
    dominant_process = process_counter.most_common(1)[0][0] if process_counter else ""
    suggestions = build_suggestions(dominant_intent)

    result = {
        "goal_hint": goal,
        "dominant_intent": dominant_intent,
        "intent_distribution": dict(intent_counter),
        "dominant_window_title": dominant_window,
        "dominant_process": dominant_process,
        "samples_collected": len(sample_rows),
        "duration_sec": round(time.time() - started_at, 2),
        "suggested_commands": suggestions,
        "ocr_errors": list(dict.fromkeys(ocr_errors))[:2],
        "samples": sample_rows,
    }

    output = json.dumps(result, ensure_ascii=False)
    print(output, flush=True)
    log(f"INTENT_RESULT_JSON={output}")

if __name__ == "__main__":
    main()
"#,
  )?;

  ensure_default_script(
    &scripts_dir.join("screen_behavior_watch.py"),
    r#"import ctypes
import ctypes.wintypes as wintypes
import datetime
import json
import os
import sys
import time
from collections import Counter

PROCESS_QUERY_LIMITED_INFORMATION = 0x1000

BEHAVIOR_TO_SUGGESTIONS = {
    "reading_or_idle": [
        "screen intent research",
        "watch screen <keyword>",
    ],
    "watching_content": [
        "watch screen <keyword>",
        "open app edge",
    ],
    "navigation_ui": [
        "open folder downloads",
        "human click <x,y>",
    ],
    "interactive_editing": [
        "human type <text>",
        "run skill desktop_skill_ops hotkey:ctrl,s",
    ],
    "multitask_switching": [
        "latest screen intent",
        "watch screen behavior workflow",
    ],
}

class POINT(ctypes.Structure):
    _fields_ = [("x", wintypes.LONG), ("y", wintypes.LONG)]

def log(msg: str):
    now = datetime.datetime.now().isoformat(timespec="seconds")
    print(f"[{now}] {msg}", flush=True)

def parse_options(raw: str):
    defaults = {
        "goal": "workflow",
        "duration": "16",
        "interval": "1.0",
        "samples": "10",
        "screen": "1",
    }
    text = (raw or "").strip()
    if not text:
        return defaults

    parts = [p.strip() for p in text.split() if p.strip()]
    if parts and "=" not in parts[0]:
        defaults["goal"] = parts[0]

    for part in parts:
        if "=" not in part:
            continue
        key, value = part.split("=", 1)
        key = key.strip().lower()
        value = value.strip()
        if key in defaults and value:
            defaults[key] = value
    return defaults

def parse_float(value: str, default: float, min_v: float, max_v: float):
    try:
        parsed = float(value)
    except Exception:
        return default
    return max(min_v, min(max_v, parsed))

def parse_int(value: str, default: int, min_v: int, max_v: int):
    try:
        parsed = int(value)
    except Exception:
        return default
    return max(min_v, min(max_v, parsed))

def parse_bool(value: str):
    return str(value).strip().lower() in {"1", "true", "yes", "on"}

def resolve_process_name(pid: int):
    if not pid:
        return ""
    try:
        kernel32 = ctypes.windll.kernel32
        kernel32.OpenProcess.argtypes = [wintypes.DWORD, wintypes.BOOL, wintypes.DWORD]
        kernel32.OpenProcess.restype = wintypes.HANDLE
        kernel32.QueryFullProcessImageNameW.argtypes = [
            wintypes.HANDLE,
            wintypes.DWORD,
            wintypes.LPWSTR,
            ctypes.POINTER(wintypes.DWORD),
        ]
        kernel32.QueryFullProcessImageNameW.restype = wintypes.BOOL
        kernel32.CloseHandle.argtypes = [wintypes.HANDLE]
        kernel32.CloseHandle.restype = wintypes.BOOL

        handle = kernel32.OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, False, pid)
        if not handle:
            return ""
        try:
            size = wintypes.DWORD(1024)
            buffer = ctypes.create_unicode_buffer(size.value)
            ok = kernel32.QueryFullProcessImageNameW(handle, 0, buffer, ctypes.byref(size))
            if not ok:
                return ""
            return os.path.basename(buffer.value).lower()
        finally:
            kernel32.CloseHandle(handle)
    except Exception:
        return ""

def foreground_window_info():
    try:
        user32 = ctypes.windll.user32
        hwnd = user32.GetForegroundWindow()
        if not hwnd:
            return {"title": "", "process": "", "pid": 0}

        length = user32.GetWindowTextLengthW(hwnd)
        length = max(0, min(length, 2048))
        buffer = ctypes.create_unicode_buffer(length + 1)
        user32.GetWindowTextW(hwnd, buffer, len(buffer))
        title = buffer.value.strip()

        pid = wintypes.DWORD(0)
        user32.GetWindowThreadProcessId(hwnd, ctypes.byref(pid))
        process_name = resolve_process_name(pid.value)
        return {"title": title, "process": process_name, "pid": int(pid.value)}
    except Exception:
        return {"title": "", "process": "", "pid": 0}

def get_cursor_pos():
    pt = POINT()
    ok = ctypes.windll.user32.GetCursorPos(ctypes.byref(pt))
    if not ok:
        return None
    return (int(pt.x), int(pt.y))

def cursor_distance(a, b):
    if not a or not b:
        return 0.0
    dx = b[0] - a[0]
    dy = b[1] - a[1]
    return (dx * dx + dy * dy) ** 0.5

def build_suggestions(dominant_behavior: str):
    return BEHAVIOR_TO_SUGGESTIONS.get(
        dominant_behavior,
        ["screen intent workflow", "watch screen behavior workflow"],
    )

def classify_step_behavior(cursor_delta: float, motion_index: float, switched_window: bool):
    moving = cursor_delta >= 18.0
    dynamic_screen = motion_index >= 0.045
    if switched_window and moving:
        return "multitask_switching"
    if moving and dynamic_screen:
        return "interactive_editing"
    if moving and not dynamic_screen:
        return "navigation_ui"
    if (not moving) and dynamic_screen:
        return "watching_content"
    return "reading_or_idle"

def main():
    raw = sys.argv[1] if len(sys.argv) > 1 else ""
    opts = parse_options(raw)
    goal = opts["goal"]
    duration = parse_float(opts["duration"], default=16.0, min_v=4.0, max_v=120.0)
    interval = parse_float(opts["interval"], default=1.0, min_v=0.2, max_v=10.0)
    samples = parse_int(opts["samples"], default=10, min_v=1, max_v=120)
    enable_screen_motion = parse_bool(opts["screen"])

    max_by_duration = max(1, int(duration / interval))
    total_samples = min(samples, max_by_duration)

    log(
        f"screen_behavior_watch start goal={goal} duration={duration}s interval={interval}s "
        f"samples={total_samples} screen_motion={enable_screen_motion}"
    )

    capture_screen_motion = False
    mss_module = None
    image_module = None
    imagechops_module = None
    imagestat_module = None
    if enable_screen_motion:
        try:
            import mss  # type: ignore
            from PIL import Image, ImageChops, ImageStat  # type: ignore
            mss_module = mss
            image_module = Image
            imagechops_module = ImageChops
            imagestat_module = ImageStat
            capture_screen_motion = True
        except Exception as error:
            log("screen motion dependencies missing, fallback to cursor+window mode.")
            log("install with: pip install mss pillow")
            log(f"import error: {error}")
            capture_screen_motion = False

    started_at = time.time()
    prev_cursor = get_cursor_pos()
    prev_window_title = ""
    prev_frame_gray = None
    behavior_counter = Counter()
    process_counter = Counter()
    window_counter = Counter()
    total_cursor_distance = 0.0
    motion_values = []
    rows = []

    if capture_screen_motion:
        sct_ctx = mss_module.mss()
        monitor = sct_ctx.monitors[1]
    else:
        sct_ctx = None
        monitor = None

    try:
        for idx in range(total_samples):
            info = foreground_window_info()
            window_title = info.get("title", "")
            process_name = info.get("process", "")

            cursor_pos = get_cursor_pos()
            cursor_delta = cursor_distance(prev_cursor, cursor_pos)
            total_cursor_distance += cursor_delta
            prev_cursor = cursor_pos

            motion_index = 0.0
            if capture_screen_motion and sct_ctx is not None and monitor is not None:
                try:
                    shot = sct_ctx.grab(monitor)
                    frame = image_module.frombytes("RGB", shot.size, shot.rgb).convert("L")
                    if prev_frame_gray is not None:
                        diff = imagechops_module.difference(prev_frame_gray, frame)
                        stat = imagestat_module.Stat(diff)
                        motion_index = float(stat.mean[0]) / 255.0
                    prev_frame_gray = frame
                except Exception:
                    motion_index = 0.0

            switched_window = bool(prev_window_title) and window_title != prev_window_title
            prev_window_title = window_title

            step_behavior = classify_step_behavior(cursor_delta, motion_index, switched_window)
            behavior_counter[step_behavior] += 1
            if process_name:
                process_counter[process_name] += 1
            if window_title:
                window_counter[window_title] += 1
            motion_values.append(motion_index)

            rows.append(
                {
                    "index": idx + 1,
                    "timestamp": datetime.datetime.now().isoformat(timespec="seconds"),
                    "window_title": window_title,
                    "process": process_name,
                    "cursor": {"x": cursor_pos[0], "y": cursor_pos[1]} if cursor_pos else None,
                    "cursor_delta": round(cursor_delta, 2),
                    "motion_index": round(motion_index, 5),
                    "step_behavior": step_behavior,
                }
            )

            log(
                f"sample {idx + 1}/{total_samples} process={process_name or '-'} "
                f"cursor_delta={cursor_delta:.1f} motion={motion_index:.4f} behavior={step_behavior}"
            )

            if idx < total_samples - 1:
                elapsed = time.time() - started_at
                if elapsed + interval > duration:
                    break
                time.sleep(interval)
    finally:
        if sct_ctx is not None:
            sct_ctx.close()

    dominant_behavior = behavior_counter.most_common(1)[0][0] if behavior_counter else "unknown"
    dominant_process = process_counter.most_common(1)[0][0] if process_counter else ""
    dominant_window = window_counter.most_common(1)[0][0] if window_counter else ""
    motion_index_avg = (sum(motion_values) / len(motion_values)) if motion_values else 0.0

    result = {
        "goal_hint": goal,
        "dominant_behavior": dominant_behavior,
        "behavior_distribution": dict(behavior_counter),
        "dominant_process": dominant_process,
        "dominant_window_title": dominant_window,
        "samples_collected": len(rows),
        "duration_sec": round(time.time() - started_at, 2),
        "motion_index": round(motion_index_avg, 5),
        "cursor_distance": round(total_cursor_distance, 2),
        "suggested_commands": build_suggestions(dominant_behavior),
        "samples": rows,
    }

    output = json.dumps(result, ensure_ascii=False)
    print(output, flush=True)
    log(f"BEHAVIOR_RESULT_JSON={output}")

if __name__ == "__main__":
    main()
"#,
  )?;

  ensure_default_script(
    &scripts_dir.join("human_input_ops.py"),
    r#"import datetime
import math
import random
import sys
import time

def log(msg: str):
    now = datetime.datetime.now().isoformat(timespec="seconds")
    print(f"[{now}] {msg}", flush=True)

def blocked_command(raw: str) -> bool:
    lowered = raw.lower().replace(" ", "")
    blocked = [
        "hotkey:alt,f4",
        "hotkey:win,r",
        "hotkey:win,x",
        "hotkey:ctrl,alt,del",
        "press:delete",
    ]
    return any(lowered.startswith(item) for item in blocked)

def parse_pair(text: str):
    values = [v.strip() for v in text.split(",", 1)]
    if len(values) != 2:
        return None
    try:
        return int(values[0]), int(values[1])
    except Exception:
        return None

def parse_drag(text: str):
    sep = ">" if ">" in text else "->" if "->" in text else None
    if not sep:
        return None
    left, right = text.split(sep, 1)
    p1 = parse_pair(left.strip())
    p2 = parse_pair(right.strip())
    if not p1 or not p2:
        return None
    return p1[0], p1[1], p2[0], p2[1]

def clamp(value: float, low: float, high: float):
    return max(low, min(high, value))

def fitts_duration(distance: float, width: float = 120.0):
    width = max(20.0, width)
    idx = math.log2(distance / width + 1.0) if distance > 0 else 0.0
    mt = 0.15 + 0.18 * idx
    return clamp(mt, 0.15, 1.4)

def cubic_bezier(t, p0, p1, p2, p3):
    one = 1.0 - t
    x = (
        one * one * one * p0[0]
        + 3 * one * one * t * p1[0]
        + 3 * one * t * t * p2[0]
        + t * t * t * p3[0]
    )
    y = (
        one * one * one * p0[1]
        + 3 * one * one * t * p1[1]
        + 3 * one * t * t * p2[1]
        + t * t * t * p3[1]
    )
    return x, y

def move_human(pyautogui, tx: int, ty: int, duration: float = None):
    sx, sy = pyautogui.position()
    dx = tx - sx
    dy = ty - sy
    distance = (dx * dx + dy * dy) ** 0.5
    if duration is None:
        duration = fitts_duration(distance)
    steps = max(14, min(120, int(duration * 75)))

    base_angle = math.atan2(dy, dx if abs(dx) > 0.001 else 0.001)
    perpendicular = base_angle + math.pi / 2
    spread = clamp(distance * 0.18, 18.0, 180.0)
    jitter1 = random.uniform(-spread, spread)
    jitter2 = random.uniform(-spread, spread)

    c1 = (
        sx + dx * 0.33 + math.cos(perpendicular) * jitter1,
        sy + dy * 0.33 + math.sin(perpendicular) * jitter1,
    )
    c2 = (
        sx + dx * 0.66 + math.cos(perpendicular) * jitter2,
        sy + dy * 0.66 + math.sin(perpendicular) * jitter2,
    )

    dt = duration / steps
    for i in range(1, steps + 1):
        t = i / steps
        x, y = cubic_bezier(t, (sx, sy), c1, c2, (tx, ty))
        pyautogui.moveTo(int(round(x)), int(round(y)), _pause=False)
        time.sleep(dt)

def main():
    raw = sys.argv[1] if len(sys.argv) > 1 else ""
    if not raw:
        log("usage: move:x,y | click[:x,y] | doubleclick[:x,y] | drag:x1,y1>x2,y2 | type:text | press:key | hotkey:key1,key2")
        raise SystemExit(1)

    if blocked_command(raw):
        log(f"blocked dangerous command: {raw}")
        raise SystemExit(2)

    try:
        import pyautogui
    except Exception as e:
        log("missing dependency. install with:")
        log("pip install pyautogui")
        log(f"import error: {e}")
        raise SystemExit(1)

    pyautogui.FAILSAFE = True
    pyautogui.PAUSE = 0.04

    cmd = raw.strip()
    lowered = cmd.lower()
    log(f"human_input_ops received: {cmd}")

    if lowered.startswith("move:"):
        pair = parse_pair(cmd.split(":", 1)[1].strip())
        if not pair:
            log("invalid move format, expected move:x,y")
            raise SystemExit(1)
        move_human(pyautogui, pair[0], pair[1], duration=None)
        log("ok human move")
        return

    if lowered.startswith("click:"):
        pair = parse_pair(cmd.split(":", 1)[1].strip())
        if not pair:
            log("invalid click format, expected click:x,y")
            raise SystemExit(1)
        move_human(pyautogui, pair[0], pair[1], duration=None)
        pyautogui.click()
        log("ok human click")
        return

    if lowered == "click":
        pyautogui.click()
        log("ok click")
        return

    if lowered.startswith("doubleclick:"):
        pair = parse_pair(cmd.split(":", 1)[1].strip())
        if not pair:
            log("invalid doubleclick format, expected doubleclick:x,y")
            raise SystemExit(1)
        move_human(pyautogui, pair[0], pair[1], duration=None)
        pyautogui.doubleClick()
        log("ok human doubleclick")
        return

    if lowered == "doubleclick":
        pyautogui.doubleClick()
        log("ok doubleclick")
        return

    if lowered.startswith("drag:"):
        parsed = parse_drag(cmd.split(":", 1)[1].strip())
        if not parsed:
            log("invalid drag format, expected drag:x1,y1>x2,y2")
            raise SystemExit(1)
        x1, y1, x2, y2 = parsed
        move_human(pyautogui, x1, y1, duration=None)
        pyautogui.mouseDown()
        try:
            move_human(pyautogui, x2, y2, duration=fitts_duration(((x2 - x1) ** 2 + (y2 - y1) ** 2) ** 0.5) * 1.15)
        finally:
            pyautogui.mouseUp()
        log("ok human drag")
        return

    if lowered.startswith("type:"):
        text = cmd.split(":", 1)[1]
        if not text:
            log("empty type text")
            raise SystemExit(1)
        for ch in text:
            pyautogui.typewrite(ch)
            time.sleep(random.uniform(0.02, 0.11))
        log("ok human type")
        return

    if lowered.startswith("press:"):
        key = cmd.split(":", 1)[1].strip()
        pyautogui.press(key)
        log(f"ok press {key}")
        return

    if lowered.startswith("hotkey:"):
        keys = [k.strip() for k in cmd.split(":", 1)[1].split(",") if k.strip()]
        if not keys:
            log("empty hotkey sequence")
            raise SystemExit(1)
        pyautogui.hotkey(*keys)
        log(f"ok hotkey {keys}")
        return

    if lowered.startswith("wait:"):
        try:
            seconds = float(cmd.split(":", 1)[1].strip())
        except Exception:
            seconds = 0.5
        seconds = clamp(seconds, 0.0, 8.0)
        time.sleep(seconds)
        log(f"ok wait {seconds}")
        return

    log(f"unknown command: {cmd}")
    raise SystemExit(1)

if __name__ == '__main__':
    main()
"#,
  )?;

  ensure_default_script(
    &scripts_dir.join("safe_desktop_action.py"),
    r#"import datetime
import sys

def log(msg: str):
    now = datetime.datetime.now().isoformat(timespec="seconds")
    print(f"[{now}] {msg}", flush=True)

def blocked_command(raw: str) -> bool:
    lowered = raw.lower().replace(" ", "")
    blocked = [
        "hotkey:alt,f4",
        "hotkey:win,r",
        "hotkey:win,x",
        "hotkey:ctrl,alt,del",
        "press:delete",
    ]
    return any(lowered.startswith(item) for item in blocked)

def main():
    raw = sys.argv[1] if len(sys.argv) > 1 else ""
    if not raw:
        log("usage: move:x,y | click | doubleclick | type:text | press:key | hotkey:key1,key2")
        raise SystemExit(1)

    if blocked_command(raw):
        log(f"blocked dangerous command: {raw}")
        raise SystemExit(2)

    try:
        import pyautogui
    except Exception as e:
        log("missing dependency. install with:")
        log("pip install pyautogui")
        log(f"import error: {e}")
        raise SystemExit(1)

    pyautogui.FAILSAFE = True
    pyautogui.PAUSE = 0.08

    cmd = raw.strip()
    lowered = cmd.lower()
    log(f"safe_desktop_action received: {cmd}")

    if lowered.startswith("move:"):
        values = cmd.split(":", 1)[1]
        x_text, y_text = [v.strip() for v in values.split(",", 1)]
        pyautogui.moveTo(int(x_text), int(y_text), duration=0.2)
        log("ok move")
        return

    if lowered == "click":
        pyautogui.click()
        log("ok click")
        return

    if lowered == "doubleclick":
        pyautogui.doubleClick()
        log("ok doubleclick")
        return

    if lowered.startswith("type:"):
        text = cmd.split(":", 1)[1]
        pyautogui.typewrite(text, interval=0.02)
        log("ok type")
        return

    if lowered.startswith("press:"):
        key = cmd.split(":", 1)[1].strip()
        pyautogui.press(key)
        log(f"ok press {key}")
        return

    if lowered.startswith("hotkey:"):
        keys = [k.strip() for k in cmd.split(":", 1)[1].split(",") if k.strip()]
        pyautogui.hotkey(*keys)
        log(f"ok hotkey {keys}")
        return

    log(f"unknown command: {cmd}")
    raise SystemExit(1)

if __name__ == '__main__':
    main()
"#,
  )?;

  ensure_default_script(
    &scripts_dir.join("desktop_skill_ops.py"),
    r#"import datetime
import sys
import time

def log(msg: str):
    now = datetime.datetime.now().isoformat(timespec="seconds")
    print(f"[{now}] {msg}", flush=True)

def blocked_command(raw: str) -> bool:
    lowered = raw.lower().replace(" ", "")
    blocked = [
        "hotkey:alt,f4",
        "hotkey:win,r",
        "hotkey:win,x",
        "hotkey:ctrl,alt,del",
        "press:delete",
    ]
    return any(lowered.startswith(item) for item in blocked)

def main():
    raw = sys.argv[1] if len(sys.argv) > 1 else ""
    if not raw:
        log("usage: move:x,y | click | doubleclick | rightclick | scroll:n | type:text | press:key | hotkey:key1,key2 | wait:seconds")
        raise SystemExit(1)

    if blocked_command(raw):
        log(f"blocked dangerous command: {raw}")
        raise SystemExit(2)

    try:
        import pyautogui
    except Exception as e:
        log("missing dependency. install with:")
        log("pip install pyautogui")
        log(f"import error: {e}")
        raise SystemExit(1)

    pyautogui.FAILSAFE = True
    pyautogui.PAUSE = 0.08

    cmd = raw.strip()
    lowered = cmd.lower()
    log(f"desktop_skill_ops received: {cmd}")

    if lowered.startswith("move:"):
        values = cmd.split(":", 1)[1]
        x_text, y_text = [v.strip() for v in values.split(",", 1)]
        pyautogui.moveTo(int(x_text), int(y_text), duration=0.2)
        log("ok move")
        return

    if lowered == "click":
        pyautogui.click()
        log("ok click")
        return

    if lowered == "doubleclick":
        pyautogui.doubleClick()
        log("ok doubleclick")
        return

    if lowered == "rightclick":
        pyautogui.rightClick()
        log("ok rightclick")
        return

    if lowered.startswith("scroll:"):
        amount = int(cmd.split(":", 1)[1].strip())
        pyautogui.scroll(amount)
        log(f"ok scroll {amount}")
        return

    if lowered.startswith("wait:"):
        seconds = float(cmd.split(":", 1)[1].strip())
        seconds = min(5.0, max(0.0, seconds))
        time.sleep(seconds)
        log(f"ok wait {seconds}")
        return

    if lowered.startswith("type:"):
        text = cmd.split(":", 1)[1]
        pyautogui.typewrite(text, interval=0.02)
        log("ok type")
        return

    if lowered.startswith("press:"):
        key = cmd.split(":", 1)[1].strip()
        pyautogui.press(key)
        log(f"ok press {key}")
        return

    if lowered.startswith("hotkey:"):
        keys = [k.strip() for k in cmd.split(":", 1)[1].split(",") if k.strip()]
        pyautogui.hotkey(*keys)
        log(f"ok hotkey {keys}")
        return

    log(f"unknown command: {cmd}")
    raise SystemExit(1)

if __name__ == '__main__':
    main()
"#,
  )?;

  Ok(())
}

fn load_local_skills() -> Vec<LocalSkillDefinition> {
  if let Err(error) = ensure_skills_dir() {
    eprintln!("failed to ensure skills dir: {error}");
    return Vec::new();
  }

  let dir = skills_dir_path();
  let Ok(entries) = fs::read_dir(&dir) else {
    return Vec::new();
  };

  let mut skills = Vec::new();
  for entry in entries.flatten() {
    let path = entry.path();
    if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
      continue;
    }
    let Ok(content) = fs::read_to_string(&path) else {
      continue;
    };
    let Ok(skill) = serde_json::from_str::<LocalSkillDefinition>(&content) else {
      continue;
    };
    if skill.id.trim().is_empty() || skill.kind.trim().is_empty() || skill.target_template.trim().is_empty()
    {
      continue;
    }
    skills.push(skill);
  }

  skills.sort_by(|a, b| a.id.cmp(&b.id));
  skills
}

fn default_local_skills() -> Vec<LocalSkillDefinition> {
  vec![
    LocalSkillDefinition {
      id: "open_github".into(),
      name: "Open GitHub".into(),
      description: "Open GitHub homepage.".into(),
      kind: "open_url".into(),
      target_template: "https://github.com".into(),
      label_template: Some("GitHub".into()),
      risk_level: Some("low-risk".into()),
      aliases: Some(vec!["github".into(), "代码仓库".into()]),
    },
    LocalSkillDefinition {
      id: "open_tradingview".into(),
      name: "Open TradingView".into(),
      description: "Open TradingView market chart.".into(),
      kind: "open_url".into(),
      target_template: "https://www.tradingview.com/chart/".into(),
      label_template: Some("TradingView chart".into()),
      risk_level: Some("low-risk".into()),
      aliases: Some(vec!["tv".into(), "股票图表".into()]),
    },
    LocalSkillDefinition {
      id: "search_stock_news".into(),
      name: "Search Stock News".into(),
      description: "Search stock news by keyword input.".into(),
      kind: "search_web".into(),
      target_template: "{{input}} stock news".into(),
      label_template: Some("Stock news: {{input}}".into()),
      risk_level: Some("low-risk".into()),
      aliases: Some(vec!["stocknews".into(), "股票新闻".into()]),
    },
    LocalSkillDefinition {
      id: "open_qmdownload".into(),
      name: "Open QMDownload".into(),
      description: "Open D:\\QMDownload folder.".into(),
      kind: "open_folder".into(),
      target_template: r"D:\QMDownload".into(),
      label_template: Some("QMDownload folder".into()),
      risk_level: Some("low-risk".into()),
      aliases: Some(vec!["downloads".into(), "下载目录".into()]),
    },
    LocalSkillDefinition {
      id: "open_firefox".into(),
      name: "Open Firefox".into(),
      description: "Launch Mozilla Firefox browser.".into(),
      kind: "open_app".into(),
      target_template: "firefox".into(),
      label_template: Some("Mozilla Firefox".into()),
      risk_level: Some("low-risk".into()),
      aliases: Some(vec!["firefox".into(), "mozilla".into()]),
    },
    LocalSkillDefinition {
      id: "open_vscode".into(),
      name: "Open VS Code".into(),
      description: "Launch Visual Studio Code.".into(),
      kind: "open_app".into(),
      target_template: "vscode".into(),
      label_template: Some("Visual Studio Code".into()),
      risk_level: Some("low-risk".into()),
      aliases: Some(vec!["vscode".into(), "code".into()]),
    },
    LocalSkillDefinition {
      id: "open_terminal".into(),
      name: "Open Terminal".into(),
      description: "Launch a PowerShell terminal window.".into(),
      kind: "open_app".into(),
      target_template: "powershell".into(),
      label_template: Some("PowerShell".into()),
      risk_level: Some("low-risk".into()),
      aliases: Some(vec!["terminal".into(), "powershell".into()]),
    },
    LocalSkillDefinition {
      id: "open_music_player".into(),
      name: "Open Music Player".into(),
      description: "Launch a local music player (Spotify/VLC/WMP fallback).".into(),
      kind: "open_app".into(),
      target_template: "music".into(),
      label_template: Some("Music player".into()),
      risk_level: Some("low-risk".into()),
      aliases: Some(vec!["music".into(), "musicplayer".into()]),
    },
    LocalSkillDefinition {
      id: "open_spotify_app".into(),
      name: "Open Spotify".into(),
      description: "Launch Spotify desktop app.".into(),
      kind: "open_app".into(),
      target_template: "spotify".into(),
      label_template: Some("Spotify".into()),
      risk_level: Some("low-risk".into()),
      aliases: Some(vec!["spotify".into()]),
    },
    LocalSkillDefinition {
      id: "screen_watch_stub".into(),
      name: "Screen Watch Stub".into(),
      description: "Run a local python stub script for screen-watch workflow.".into(),
      kind: "run_script".into(),
      target_template: "screen_watch_ocr.py".into(),
      label_template: Some("Screen Watch Stub".into()),
      risk_level: Some("medium-risk".into()),
      aliases: Some(vec!["watchscreen".into(), "盯屏".into(), "屏幕监控".into()]),
    },
    LocalSkillDefinition {
      id: "screen_watch_ocr".into(),
      name: "Screen Watch OCR".into(),
      description: "Watch screen OCR text and detect keyword hits.".into(),
      kind: "run_script".into(),
      target_template: "screen_watch_ocr.py".into(),
      label_template: Some("Screen Watch OCR".into()),
      risk_level: Some("medium-risk".into()),
      aliases: Some(vec!["watchocr".into(), "ocrwatch".into(), "盯屏识别".into()]),
    },
    LocalSkillDefinition {
      id: "screen_intent_watch".into(),
      name: "Screen Intent Watch".into(),
      description: "Observe foreground window + OCR signals and infer likely user intent.".into(),
      kind: "run_script".into(),
      target_template: "screen_intent_watch.py".into(),
      label_template: Some("Screen Intent Watch".into()),
      risk_level: Some("medium-risk".into()),
      aliases: Some(vec!["intentwatch".into(), "screenintent".into(), "watchintent".into()]),
    },
    LocalSkillDefinition {
      id: "screen_behavior_watch".into(),
      name: "Screen Behavior Watch".into(),
      description: "Observe screen dynamics + cursor movement and infer behavior state.".into(),
      kind: "run_script".into(),
      target_template: "screen_behavior_watch.py".into(),
      label_template: Some("Screen Behavior Watch".into()),
      risk_level: Some("medium-risk".into()),
      aliases: Some(vec!["behaviorwatch".into(), "screenbehavior".into(), "mousebehavior".into()]),
    },
    LocalSkillDefinition {
      id: "desktop_action_safe".into(),
      name: "Desktop Action Safe".into(),
      description: "Execute constrained mouse/keyboard actions via script input.".into(),
      kind: "run_script".into(),
      target_template: "safe_desktop_action.py".into(),
      label_template: Some("Desktop Action Safe".into()),
      risk_level: Some("high-risk".into()),
      aliases: Some(vec!["pcaction".into(), "桌面操作".into(), "键鼠操作".into()]),
    },
    LocalSkillDefinition {
      id: "desktop_skill_ops".into(),
      name: "Desktop Skill Ops".into(),
      description: "Richer desktop skill actions: move/click/right-click/scroll/type/hotkey.".into(),
      kind: "run_script".into(),
      target_template: "desktop_skill_ops.py".into(),
      label_template: Some("Desktop Skill Ops".into()),
      risk_level: Some("high-risk".into()),
      aliases: Some(vec!["desktopops".into(), "mouseops".into(), "keyboardops".into()]),
    },
    LocalSkillDefinition {
      id: "human_input_ops".into(),
      name: "Human Input Ops".into(),
      description: "Human-like mouse and keyboard operations (smooth move/click/drag/type).".into(),
      kind: "run_script".into(),
      target_template: "human_input_ops.py".into(),
      label_template: Some("Human Input Ops".into()),
      risk_level: Some("high-risk".into()),
      aliases: Some(vec!["humaninput".into(), "humanmouse".into(), "smoothinput".into()]),
    },
  ]
}

fn unsupported_parameter_plan(topic: &str, value: &str, hint: &str) -> CommandPlan {
  CommandPlan {
    assistant_reply: format!("I could not resolve this {topic}: \"{value}\"."),
    risk_level: "not-implemented".into(),
    can_execute_directly: false,
    steps: vec![
      step("plan-param-1", "Read request", "Parsed parameterized command", "done"),
      step("plan-param-2", "Resolve parameter", "Parameter value was not recognized", "done"),
      step("plan-param-3", "Recovery hint", hint, "waiting"),
    ],
    suggested_action: None,
  }
}

fn open_folder(target: &str, label: &str) -> Result<(String, Vec<String>), String> {
  if !Path::new(target).exists() {
    return Err(format!("Folder path does not exist: {target}"));
  }

  Command::new("explorer")
    .arg(target)
    .spawn()
    .map_err(|error| format!("Failed to open folder: {error}"))?;

  Ok((
    format!("Opened {label}."),
    vec![target.into(), "Executed through Windows Explorer".into()],
  ))
}

fn open_url(target: &str, label: &str) -> Result<(String, Vec<String>), String> {
  Command::new("cmd")
    .args(["/C", "start", "", target])
    .spawn()
    .map_err(|error| format!("Failed to open url: {error}"))?;

  Ok((
    format!("Opened {label}."),
    vec![target.into(), "Executed through the default browser".into()],
  ))
}

fn open_app(target: &str, label: &str) -> Result<(String, Vec<String>), String> {
  let launched = match target {
    "chrome" => try_spawn_any(&[
      r"C:\Program Files\Google\Chrome\Application\chrome.exe",
      r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
      "chrome.exe",
    ]),
    "edge" => try_spawn_any(&[
      r"C:\Program Files\Microsoft\Edge\Application\msedge.exe",
      r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
      "msedge.exe",
    ]),
    "firefox" => try_spawn_any(&[
      r"C:\Program Files\Mozilla Firefox\firefox.exe",
      r"C:\Program Files (x86)\Mozilla Firefox\firefox.exe",
      "firefox.exe",
    ]),
    "vscode" => try_spawn_vscode(),
    "powershell" | "terminal" => Command::new("powershell.exe").spawn().is_ok(),
    "cmd" => Command::new("cmd.exe").spawn().is_ok(),
    "taskmgr" => Command::new("taskmgr.exe").spawn().is_ok(),
    "spotify" => try_spawn_spotify(),
    "vlc" => try_spawn_any(&[
      r"C:\Program Files\VideoLAN\VLC\vlc.exe",
      r"C:\Program Files (x86)\VideoLAN\VLC\vlc.exe",
      "vlc.exe",
    ]),
    "wmplayer" => try_spawn_any(&[
      r"C:\Program Files\Windows Media Player\wmplayer.exe",
      "wmplayer.exe",
    ]),
    "music" => try_spawn_music_player(),
    "notepad" => Command::new("notepad.exe").spawn().is_ok(),
    "explorer" => Command::new("explorer.exe").spawn().is_ok(),
    "calculator" => Command::new("calc.exe").spawn().is_ok(),
    "paint" => Command::new("mspaint.exe").spawn().is_ok(),
    _ => false,
  };

  if !launched {
    return Err(format!("Failed to launch {label}."));
  }

  Ok((
    format!("Launched {label}."),
    vec![format!("target={target}")],
  ))
}

fn run_script(target: &str, label: &str) -> Result<(String, Vec<String>), String> {
  let payload = parse_script_target(target);
  let script_path = resolve_skill_script_path(&payload.script)?;
  let extension = script_path
    .extension()
    .and_then(|ext| ext.to_str())
    .unwrap_or_default()
    .to_lowercase();

  let mut command = match extension.as_str() {
    "py" => {
      let mut cmd = Command::new("python");
      cmd.arg(&script_path);
      cmd
    }
    "ps1" => {
      let mut cmd = Command::new("powershell");
      cmd.args(["-ExecutionPolicy", "Bypass", "-File"]);
      cmd.arg(&script_path);
      cmd
    }
    _ => {
      return Err("Only .py and .ps1 scripts are supported for run_script skills.".into());
    }
  };

  if let Some(input) = payload.input.as_ref().filter(|value| !value.trim().is_empty()) {
    command.arg(input);
  }

  let run_id = now_unix_ms();
  let run_log_path = prepare_script_run_log_path(&script_path, run_id)?;
  let stdout_file = File::create(&run_log_path)
    .map_err(|error| format!("Failed to create script run log: {error}"))?;
  let stderr_file = stdout_file
    .try_clone()
    .map_err(|error| format!("Failed to clone script run log handle: {error}"))?;

  command
    .stdout(Stdio::from(stdout_file))
    .stderr(Stdio::from(stderr_file));

  let child = command
    .spawn()
    .map_err(|error| format!("Failed to start script: {error}"))?;

  let mut details = vec![
    format!("script={}", script_path.to_string_lossy()),
    format!("pid={}", child.id()),
    format!("runner={extension}"),
    format!("run_log={}", run_log_path.to_string_lossy()),
  ];

  if let Some(input) = payload.input {
    details.push(format!("input={input}"));
  }

  Ok((format!("Started script skill {label}."), details))
}

fn read_latest_screen_intent_report(target: &str, label: &str) -> Result<(String, Vec<String>), String> {
  let runs_dir = skills_runs_dir();
  let entries = fs::read_dir(&runs_dir)
    .map_err(|error| format!("Cannot read script runs folder {}: {error}", runs_dir.to_string_lossy()))?;

  let mut latest: Option<(SystemTime, PathBuf)> = None;
  for entry in entries.flatten() {
    let path = entry.path();
    if path.extension().and_then(|ext| ext.to_str()) != Some("log") {
      continue;
    }
    let filename = path
      .file_name()
      .and_then(|name| name.to_str())
      .unwrap_or_default()
      .to_lowercase();
    if !filename.contains("screen_intent_watch") {
      continue;
    }
    let modified = entry
      .metadata()
      .ok()
      .and_then(|meta| meta.modified().ok())
      .unwrap_or(UNIX_EPOCH);

    let should_replace = match latest.as_ref() {
      Some((current_modified, _)) => modified > *current_modified,
      None => true,
    };

    if should_replace {
      latest = Some((modified, path));
    }
  }

  let (_, latest_log_path) = latest.ok_or_else(|| {
    "No screen intent logs found yet. Run `screen intent` first and wait for completion.".to_string()
  })?;

  let content = fs::read_to_string(&latest_log_path)
    .map_err(|error| format!("Cannot read intent run log {}: {error}", latest_log_path.to_string_lossy()))?;

  let mut result_json = None;
  for line in content.lines().rev() {
    if let Some((_, value)) = line.split_once("INTENT_RESULT_JSON=") {
      result_json = Some(value.trim().to_string());
      break;
    }
    let trimmed = line.trim();
    if trimmed.starts_with('{') && trimmed.contains("\"dominant_intent\"") {
      result_json = Some(trimmed.to_string());
      break;
    }
  }

  let result_json = result_json.ok_or_else(|| {
    "Latest intent run log does not contain INTENT_RESULT_JSON yet. Wait for script to finish and retry."
      .to_string()
  })?;

  let parsed: serde_json::Value = serde_json::from_str(&result_json)
    .map_err(|error| format!("Failed to parse intent report JSON from run log: {error}"))?;

  let dominant_intent = parsed
    .get("dominant_intent")
    .and_then(|value| value.as_str())
    .unwrap_or("unknown");
  let dominant_process = parsed
    .get("dominant_process")
    .and_then(|value| value.as_str())
    .unwrap_or("");
  let dominant_window = parsed
    .get("dominant_window_title")
    .and_then(|value| value.as_str())
    .unwrap_or("");
  let samples_collected = parsed
    .get("samples_collected")
    .and_then(|value| value.as_u64())
    .unwrap_or(0);

  let mut details = vec![
    format!("source={target}"),
    format!("run_log={}", latest_log_path.to_string_lossy()),
    format!("dominant_intent={dominant_intent}"),
    format!("samples_collected={samples_collected}"),
  ];

  if !dominant_process.trim().is_empty() {
    details.push(format!("dominant_process={dominant_process}"));
  }
  if !dominant_window.trim().is_empty() {
    details.push(format!("dominant_window_title={}", truncate_error_text(dominant_window, 160)));
  }

  if let Some(commands) = parsed
    .get("suggested_commands")
    .and_then(|value| value.as_array())
    .map(|items| {
      items
        .iter()
        .filter_map(|item| item.as_str())
        .take(4)
        .collect::<Vec<_>>()
    })
    .filter(|items| !items.is_empty())
  {
    details.push(format!("suggested_commands={}", commands.join(" | ")));
  }

  let summary = if dominant_intent == "unknown" {
    format!("{label}: no clear dominant intent yet.")
  } else {
    format!(
      "{label}: {dominant_intent} (process: {}, samples: {samples_collected}).",
      if dominant_process.is_empty() {
        "unknown"
      } else {
        dominant_process
      }
    )
  };

  Ok((summary, details))
}

fn read_latest_screen_behavior_report(target: &str, label: &str) -> Result<(String, Vec<String>), String> {
  let runs_dir = skills_runs_dir();
  let entries = fs::read_dir(&runs_dir)
    .map_err(|error| format!("Cannot read script runs folder {}: {error}", runs_dir.to_string_lossy()))?;

  let mut latest: Option<(SystemTime, PathBuf)> = None;
  for entry in entries.flatten() {
    let path = entry.path();
    if path.extension().and_then(|ext| ext.to_str()) != Some("log") {
      continue;
    }
    let filename = path
      .file_name()
      .and_then(|name| name.to_str())
      .unwrap_or_default()
      .to_lowercase();
    if !filename.contains("screen_behavior_watch") {
      continue;
    }
    let modified = entry
      .metadata()
      .ok()
      .and_then(|meta| meta.modified().ok())
      .unwrap_or(UNIX_EPOCH);

    let should_replace = match latest.as_ref() {
      Some((current_modified, _)) => modified > *current_modified,
      None => true,
    };

    if should_replace {
      latest = Some((modified, path));
    }
  }

  let (_, latest_log_path) = latest.ok_or_else(|| {
    "No screen behavior logs found yet. Run `watch screen behavior` first and wait for completion.".to_string()
  })?;

  let content = fs::read_to_string(&latest_log_path)
    .map_err(|error| format!("Cannot read behavior run log {}: {error}", latest_log_path.to_string_lossy()))?;

  let mut result_json = None;
  for line in content.lines().rev() {
    if let Some((_, value)) = line.split_once("BEHAVIOR_RESULT_JSON=") {
      result_json = Some(value.trim().to_string());
      break;
    }
    let trimmed = line.trim();
    if trimmed.starts_with('{') && trimmed.contains("\"dominant_behavior\"") {
      result_json = Some(trimmed.to_string());
      break;
    }
  }

  let result_json = result_json.ok_or_else(|| {
    "Latest behavior run log does not contain BEHAVIOR_RESULT_JSON yet. Wait for script to finish and retry."
      .to_string()
  })?;

  let parsed: serde_json::Value = serde_json::from_str(&result_json)
    .map_err(|error| format!("Failed to parse behavior report JSON from run log: {error}"))?;

  let dominant_behavior = parsed
    .get("dominant_behavior")
    .and_then(|value| value.as_str())
    .unwrap_or("unknown");
  let dominant_process = parsed
    .get("dominant_process")
    .and_then(|value| value.as_str())
    .unwrap_or("");
  let dominant_window = parsed
    .get("dominant_window_title")
    .and_then(|value| value.as_str())
    .unwrap_or("");
  let samples_collected = parsed
    .get("samples_collected")
    .and_then(|value| value.as_u64())
    .unwrap_or(0);
  let motion_index = parsed
    .get("motion_index")
    .and_then(|value| value.as_f64())
    .unwrap_or(0.0);
  let cursor_distance = parsed
    .get("cursor_distance")
    .and_then(|value| value.as_f64())
    .unwrap_or(0.0);

  let mut details = vec![
    format!("source={target}"),
    format!("run_log={}", latest_log_path.to_string_lossy()),
    format!("dominant_behavior={dominant_behavior}"),
    format!("samples_collected={samples_collected}"),
    format!("motion_index={motion_index:.4}"),
    format!("cursor_distance={cursor_distance:.1}"),
  ];

  if !dominant_process.trim().is_empty() {
    details.push(format!("dominant_process={dominant_process}"));
  }
  if !dominant_window.trim().is_empty() {
    details.push(format!("dominant_window_title={}", truncate_error_text(dominant_window, 160)));
  }

  if let Some(commands) = parsed
    .get("suggested_commands")
    .and_then(|value| value.as_array())
    .map(|items| {
      items
        .iter()
        .filter_map(|item| item.as_str())
        .take(4)
        .collect::<Vec<_>>()
    })
    .filter(|items| !items.is_empty())
  {
    details.push(format!("suggested_commands={}", commands.join(" | ")));
  }

  let summary = if dominant_behavior == "unknown" {
    format!("{label}: no clear dominant behavior yet.")
  } else {
    format!(
      "{label}: {dominant_behavior} (process: {}, samples: {samples_collected}).",
      if dominant_process.is_empty() {
        "unknown"
      } else {
        dominant_process
      }
    )
  };

  Ok((summary, details))
}

fn try_spawn_any(candidates: &[&str]) -> bool {
  candidates
    .iter()
    .any(|candidate| Command::new(candidate).spawn().is_ok())
}

fn try_spawn_strings(candidates: &[String]) -> bool {
  candidates
    .iter()
    .any(|candidate| Command::new(candidate).spawn().is_ok())
}

fn try_spawn_spotify() -> bool {
  let mut candidates = vec![
    r"C:\Program Files\Spotify\Spotify.exe".to_string(),
    r"C:\Program Files (x86)\Spotify\Spotify.exe".to_string(),
    "spotify.exe".to_string(),
  ];
  if let Ok(app_data) = env::var("APPDATA") {
    candidates.push(
      Path::new(&app_data)
        .join("Spotify")
        .join("Spotify.exe")
        .to_string_lossy()
        .into_owned(),
    );
  }
  if let Ok(local_app_data) = env::var("LOCALAPPDATA") {
    candidates.push(
      Path::new(&local_app_data)
        .join("Microsoft")
        .join("WindowsApps")
        .join("Spotify.exe")
        .to_string_lossy()
        .into_owned(),
    );
  }
  try_spawn_strings(&candidates)
}

fn try_spawn_vscode() -> bool {
  let mut candidates = vec![
    r"C:\Program Files\Microsoft VS Code\Code.exe".to_string(),
    r"C:\Program Files (x86)\Microsoft VS Code\Code.exe".to_string(),
    "code.exe".to_string(),
  ];
  if let Ok(local_app_data) = env::var("LOCALAPPDATA") {
    candidates.push(
      Path::new(&local_app_data)
        .join("Programs")
        .join("Microsoft VS Code")
        .join("Code.exe")
        .to_string_lossy()
        .into_owned(),
    );
  }
  try_spawn_strings(&candidates)
}

fn try_spawn_music_player() -> bool {
  try_spawn_spotify()
    || try_spawn_any(&[
      r"C:\Program Files\VideoLAN\VLC\vlc.exe",
      r"C:\Program Files (x86)\VideoLAN\VLC\vlc.exe",
      "vlc.exe",
    ])
    || try_spawn_any(&[
      r"C:\Program Files\Windows Media Player\wmplayer.exe",
      "wmplayer.exe",
    ])
}

fn build_chat_completions_endpoint(base_url: &str) -> String {
  let normalized = base_url.trim_end_matches('/');
  if normalized.ends_with("/chat/completions") {
    return normalized.to_string();
  }
  if normalized.eq_ignore_ascii_case("https://api.openai.com")
    || normalized.eq_ignore_ascii_case("http://api.openai.com")
  {
    return format!("{normalized}/v1/chat/completions");
  }
  if normalized.ends_with("/v1") {
    return format!("{normalized}/chat/completions");
  }
  format!("{normalized}/chat/completions")
}

fn extract_model_content(response_json: &serde_json::Value) -> Result<String, String> {
  if let Some(content) = response_json
    .pointer("/choices/0/message/content")
    .and_then(|value| value.as_str())
  {
    let trimmed = content.trim();
    if !trimmed.is_empty() {
      return Ok(trimmed.to_string());
    }
  }

  if let Some(content_parts) = response_json
    .pointer("/choices/0/message/content")
    .and_then(|value| value.as_array())
  {
    let joined = content_parts
      .iter()
      .filter_map(|part| part.get("text").and_then(|value| value.as_str()))
      .collect::<Vec<_>>()
      .join("\n")
      .trim()
      .to_string();

    if !joined.is_empty() {
      return Ok(joined);
    }
  }

  if let Some(legacy_text) = response_json
    .pointer("/choices/0/text")
    .and_then(|value| value.as_str())
  {
    let trimmed = legacy_text.trim();
    if !trimmed.is_empty() {
      return Ok(trimmed.to_string());
    }
  }

  Err("Model API response did not contain assistant text.".into())
}

fn truncate_error_text(text: &str, max_len: usize) -> String {
  if text.len() <= max_len {
    return text.to_string();
  }

  format!("{}...", &text[..max_len])
}

fn looks_like_html_response(text: &str) -> bool {
  let lowered = text.trim_start().to_lowercase();
  lowered.starts_with("<!doctype html")
    || lowered.starts_with("<html")
    || (lowered.contains("<html") && lowered.contains("</html>"))
}

fn extract_html_title(html_text: &str) -> Option<String> {
  let lowered = html_text.to_lowercase();
  let start = lowered.find("<title>")?;
  let end = lowered.find("</title>")?;
  if end <= start + 7 {
    return None;
  }
  Some(html_text[start + 7..end].trim().to_string())
}

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
  needles.iter().any(|needle| haystack.contains(needle))
}

fn extract_after_prefix(source: &str, prefixes: &[&str]) -> Option<String> {
  prefixes.iter().find_map(|prefix| {
    source.strip_prefix(prefix).map(|rest| rest.trim().to_string())
  })
}

fn extract_after_prefix_case_insensitive(source: &str, prefixes: &[&str]) -> Option<String> {
  let lowered = source.to_lowercase();
  prefixes.iter().find_map(|prefix| {
    lowered
      .strip_prefix(prefix)
      .map(|_| source[prefix.len()..].trim().to_string())
  })
}

fn parse_first_int(raw: &str) -> Option<i32> {
  raw
    .split(|ch: char| !ch.is_ascii_digit() && ch != '-')
    .find(|token| !token.is_empty())
    .and_then(|token| token.parse::<i32>().ok())
}

fn parse_coordinate_pair(raw: &str) -> Option<(i32, i32)> {
  let numbers = raw
    .split(|ch: char| !ch.is_ascii_digit() && ch != '-')
    .filter(|token| !token.is_empty())
    .filter_map(|token| token.parse::<i32>().ok())
    .take(2)
    .collect::<Vec<_>>();
  if numbers.len() < 2 {
    return None;
  }
  Some((numbers[0], numbers[1]))
}

fn parse_coordinate_quad(raw: &str) -> Option<(i32, i32, i32, i32)> {
  let numbers = raw
    .split(|ch: char| !ch.is_ascii_digit() && ch != '-')
    .filter(|token| !token.is_empty())
    .filter_map(|token| token.parse::<i32>().ok())
    .take(4)
    .collect::<Vec<_>>();
  if numbers.len() < 4 {
    return None;
  }
  Some((numbers[0], numbers[1], numbers[2], numbers[3]))
}

fn normalize_site_target(raw_target: &str) -> (String, String) {
  let target = raw_target
    .trim()
    .trim_matches('"')
    .trim_matches('\'')
    .trim();

  if target.starts_with("http://") || target.starts_with("https://") {
    return (target.into(), target.into());
  }

  let compact = target.replace(' ', "");
  if compact.contains('.') {
    return (format!("https://{compact}"), compact);
  }

  let query_url = build_search_url(target);
  (query_url, format!("web search: {target}"))
}

fn build_search_url(query: &str) -> String {
  format!("https://www.bing.com/search?q={}", query_to_url_param(query))
}

fn query_to_url_param(query: &str) -> String {
  query
    .trim()
    .split_whitespace()
    .collect::<Vec<_>>()
    .join("+")
}

fn normalize_alias(value: &str) -> String {
  value
    .trim()
    .to_lowercase()
    .replace(' ', "")
    .replace('_', "")
    .replace('-', "")
}

fn default_risk_for_kind(kind: &str) -> &'static str {
  match kind {
    "run_script" => "medium-risk",
    _ => "low-risk",
  }
}

fn parse_script_target(target: &str) -> ScriptTargetPayload {
  if let Ok(payload) = serde_json::from_str::<ScriptTargetPayload>(target) {
    return payload;
  }

  ScriptTargetPayload {
    script: target.to_string(),
    input: None,
  }
}

fn skills_scripts_dir() -> PathBuf {
  skills_dir_path().join("scripts")
}

fn skills_runs_dir() -> PathBuf {
  skills_dir_path().join("runs")
}

fn resolve_skill_script_path(script_value: &str) -> Result<PathBuf, String> {
  if script_value.trim().is_empty() {
    return Err("Script value is empty.".into());
  }

  let script_name = script_value.replace('\\', "/");
  if script_name.contains("..") {
    return Err("Script path traversal is not allowed.".into());
  }

  let scripts_dir = skills_scripts_dir();
  let scripts_dir_canonical = scripts_dir
    .canonicalize()
    .map_err(|error| format!("Cannot access scripts folder: {error}"))?;
  let candidate = scripts_dir.join(script_name);
  let candidate_canonical = candidate
    .canonicalize()
    .map_err(|error| format!("Cannot access script file: {error}"))?;

  if !candidate_canonical.starts_with(&scripts_dir_canonical) {
    return Err("Script must stay inside local skills scripts folder.".into());
  }

  Ok(candidate_canonical)
}

fn ensure_default_script(path: &Path, content: &str) -> Result<(), String> {
  if path.exists() {
    return Ok(());
  }

  fs::write(path, content).map_err(|error| format!("Failed to write default script: {error}"))?;
  Ok(())
}

fn sanitize_path_token(raw: &str) -> String {
  let token = raw
    .chars()
    .map(|ch| {
      if ch.is_ascii_alphanumeric() {
        ch
      } else {
        '_'
      }
    })
    .collect::<String>()
    .trim_matches('_')
    .to_string();

  if token.is_empty() {
    "skill_run".into()
  } else {
    token
  }
}

fn prepare_script_run_log_path(script_path: &Path, run_id: u128) -> Result<PathBuf, String> {
  let runs_dir = skills_runs_dir();
  fs::create_dir_all(&runs_dir)
    .map_err(|error| format!("Failed to create script runs folder: {error}"))?;

  let script_name = script_path
    .file_stem()
    .and_then(|name| name.to_str())
    .unwrap_or("skill");
  let filename = format!("run_{}_{}.log", run_id, sanitize_path_token(script_name));
  Ok(runs_dir.join(filename))
}

fn resolve_named_folder(query: &str) -> Option<(String, String)> {
  let normalized = normalize_alias(query);
  let user_profile = env::var("USERPROFILE").ok();

  match normalized.as_str() {
    "downloads" | "download" | "下载" | "下载区" => {
      let mut candidates = vec![PathBuf::from(r"D:\QMDownload")];
      if let Some(profile) = user_profile.as_ref() {
        candidates.push(Path::new(profile).join("Downloads"));
      }
      first_existing_path(&candidates).map(|path| (path, "Downloads folder".into()))
    }
    "desktop" | "桌面" => {
      let path = user_profile.as_ref().map(|profile| Path::new(profile).join("Desktop"))?;
      Some((path.to_string_lossy().into_owned(), "Desktop folder".into()))
    }
    "documents" | "document" | "文档" | "文件" => {
      let path = user_profile
        .as_ref()
        .map(|profile| Path::new(profile).join("Documents"))?;
      Some((path.to_string_lossy().into_owned(), "Documents folder".into()))
    }
    "pictures" | "picture" | "images" | "图片" | "照片" => {
      let path = user_profile
        .as_ref()
        .map(|profile| Path::new(profile).join("Pictures"))?;
      Some((path.to_string_lossy().into_owned(), "Pictures folder".into()))
    }
    "qmdownload" | "xiazai" | "d盘下载" => {
      Some((r"D:\QMDownload".into(), "QMDownload folder".into()))
    }
    "xixi" | "xixi项目" | "xixi目录" => Some((r"D:\QMDownload\xixi".into(), "xixi project folder".into())),
    _ => None,
  }
}

fn first_existing_path(candidates: &[PathBuf]) -> Option<String> {
  candidates
    .iter()
    .find(|path| path.exists())
    .map(|path| path.to_string_lossy().into_owned())
}

#[allow(dead_code)]
fn resolve_app_alias_legacy(query: &str) -> Option<(&'static str, &'static str)> {
  let normalized = normalize_alias(query);
  match normalized.as_str() {
    "chrome" | "谷歌" | "谷歌浏览器" => Some(("chrome", "Google Chrome")),
    "edge" | "微软浏览器" | "microsoftedge" => Some(("edge", "Microsoft Edge")),
    "notepad" | "记事本" => Some(("notepad", "Notepad")),
    "explorer" | "fileexplorer" | "资源管理器" | "文件管理器" => {
      Some(("explorer", "File Explorer"))
    }
    "calculator" | "calc" | "计算器" => Some(("calculator", "Calculator")),
    "paint" | "mspaint" | "画图" => Some(("paint", "Paint")),
    _ => None,
  }
}

fn resolve_app_alias(query: &str) -> Option<(&'static str, &'static str)> {
  let normalized = normalize_alias(query);
  match normalized.as_str() {
    "chrome" => Some(("chrome", "Google Chrome")),
    "edge" => Some(("edge", "Microsoft Edge")),
    "browser" | "defaultbrowser" => Some(("edge", "Browser (Microsoft Edge)")),
    "firefox" | "mozilla" => Some(("firefox", "Mozilla Firefox")),
    "vscode" | "code" | "visualstudiocode" => Some(("vscode", "Visual Studio Code")),
    "terminal" | "powershell" => Some(("powershell", "PowerShell")),
    "cmd" | "commandprompt" => Some(("cmd", "Command Prompt")),
    "taskmgr" | "taskmanager" => Some(("taskmgr", "Task Manager")),
    "spotify" => Some(("spotify", "Spotify")),
    "vlc" | "videolan" => Some(("vlc", "VLC Player")),
    "wmplayer" | "windowsmediaplayer" | "media" => Some(("wmplayer", "Windows Media Player")),
    "music" | "musicplayer" => Some(("music", "Music player")),
    "notepad" => Some(("notepad", "Notepad")),
    "explorer" | "fileexplorer" => Some(("explorer", "File Explorer")),
    "calculator" | "calc" => Some(("calculator", "Calculator")),
    "paint" | "mspaint" => Some(("paint", "Paint")),
    _ => None,
  }
}

fn recovery_tips_for_action(action: &LocalAction) -> Vec<String> {
  match action.kind.as_str() {
    "open_app" => vec![
      "Check whether the target app is installed on this Windows machine.".into(),
      "Try a different app alias like: chrome, edge, firefox, vscode, terminal, powershell, cmd, taskmgr, spotify, music, vlc, wmplayer, notepad, explorer, calculator, paint.".into(),
      "If needed, run the command once manually and retry from xixi.".into(),
    ],
    "open_folder" => vec![
      "Confirm the target folder exists and you have access permissions.".into(),
      "Try a known folder alias such as downloads, desktop, documents, or xixi folder.".into(),
    ],
    "open_url" | "search_web" => vec![
      "Check whether a default browser is configured on this Windows profile.".into(),
      "Try using a full URL like https://example.com.".into(),
    ],
    "run_script" => vec![
      "Check script file exists under %LOCALAPPDATA%\\xixi\\skills\\scripts.".into(),
      "Check run logs under %LOCALAPPDATA%\\xixi\\skills\\runs for stdout/stderr.".into(),
      "Only .py and .ps1 scripts are currently supported.".into(),
      "For Python scripts, ensure python is installed and available in PATH.".into(),
      "Some scripts need extra packages: mss, pillow, pytesseract, pyautogui.".into(),
    ],
    "read_intent_report" => vec![
      "Run `screen intent` first so a fresh report log is generated.".into(),
      "Wait for `screen_intent_watch.py` to finish and write INTENT_RESULT_JSON.".into(),
      "Check %LOCALAPPDATA%\\xixi\\skills\\runs for latest screen_intent_watch log.".into(),
    ],
    "read_behavior_report" => vec![
      "Run `watch screen behavior` first so a fresh behavior log is generated.".into(),
      "Wait for `screen_behavior_watch.py` to finish and write BEHAVIOR_RESULT_JSON.".into(),
      "Check %LOCALAPPDATA%\\xixi\\skills\\runs for latest screen_behavior_watch log.".into(),
    ],
    _ => vec!["Retry later or use a simpler supported command phrase.".into()],
  }
}

fn action_log_path() -> PathBuf {
  if let Ok(local_app_data) = env::var("LOCALAPPDATA") {
    return Path::new(&local_app_data).join("xixi").join("action-log.jsonl");
  }
  env::temp_dir().join("xixi").join("action-log.jsonl")
}

fn append_action_log(action: &LocalAction, result: &ActionExecutionResult) -> Result<(), String> {
  let path = action_log_path();
  if let Some(parent) = path.parent() {
    fs::create_dir_all(parent).map_err(|error| format!("Failed to prepare log folder: {error}"))?;
  }

  let mut file = OpenOptions::new()
    .create(true)
    .append(true)
    .open(&path)
    .map_err(|error| format!("Failed to open log file: {error}"))?;

  let line = json!({
    "action_id": result.action_id,
    "executed_at_ms": result.executed_at_ms,
    "duration_ms": result.duration_ms,
    "ok": result.ok,
    "summary": result.summary,
    "details": result.details,
    "recovery_tips": result.recovery_tips,
    "action": {
      "kind": action.kind,
      "target": action.target,
      "label": action.label,
    }
  });

  writeln!(file, "{line}").map_err(|error| format!("Failed to write log file: {error}"))?;
  Ok(())
}

fn now_unix_ms() -> u128 {
  SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .map(|duration| duration.as_millis())
    .unwrap_or(0)
}

fn ensure_pet_window<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> Result<(), String> {
  if app.get_webview_window("pet").is_some() {
    return Ok(());
  }

  WebviewWindowBuilder::new(app, "pet", WebviewUrl::App("index.html?pet=1".into()))
    .title("xixi pet")
    .inner_size(220.0, 220.0)
    .resizable(false)
    .decorations(false)
    .transparent(true)
    .always_on_top(true)
    .skip_taskbar(true)
    .visible(false)
    .build()
    .map_err(|error| format!("Failed to create pet window: {error}"))?;

  Ok(())
}

fn show_pet_window<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
  if let Err(error) = ensure_pet_window(app) {
    eprintln!("{error}");
    return;
  }

  if let Some(window) = app.get_webview_window("pet") {
    let _ = window.show();
    let _ = window.unminimize();
    let _ = window.set_focus();
  }
}

fn hide_pet_window<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
  if let Some(window) = app.get_webview_window("pet") {
    let _ = window.hide();
  }
}

fn show_main_window<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
  if let Some(window) = app.get_webview_window("main") {
    let _ = window.show();
    let _ = window.unminimize();
    let _ = window.set_focus();
  }
  hide_pet_window(app);
}

fn hide_main_window<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
  if let Some(window) = app.get_webview_window("main") {
    let _ = window.hide();
  }
  show_pet_window(app);
}

fn toggle_main_window<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
  if let Some(window) = app.get_webview_window("main") {
    if window.is_visible().unwrap_or(false) {
      let _ = window.hide();
      show_pet_window(app);
    } else {
      let _ = window.show();
      let _ = window.unminimize();
      let _ = window.set_focus();
      hide_pet_window(app);
    }
  }
}

fn step(id: &str, title: &str, detail: &str, state: &str) -> ActionItem {
  ActionItem {
    id: id.into(),
    title: title.into(),
    detail: detail.into(),
    state: state.into(),
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn plans_english_github_request() {
    let plan = plan_user_request("Open GitHub".to_string());
    assert!(plan.can_execute_directly);
    assert_eq!(plan.risk_level, "low-risk");
    assert_eq!(
      plan.suggested_action.as_ref().map(|action| action.kind.as_str()),
      Some("open_url")
    );
  }

  #[test]
  fn plans_chinese_github_request() {
    let plan = plan_user_request("帮我打开github".to_string());
    assert!(plan.can_execute_directly);
    assert_eq!(plan.risk_level, "low-risk");
    assert_eq!(
      plan.suggested_action.as_ref().map(|action| action.kind.as_str()),
      Some("open_url")
    );
  }

  #[test]
  fn plans_chinese_notepad_request() {
    let plan = plan_user_request("打开记事本".to_string());
    assert!(plan.can_execute_directly);
    assert_eq!(
      plan.suggested_action.as_ref().map(|action| action.kind.as_str()),
      Some("open_app")
    );
    assert_eq!(
      plan.suggested_action
        .as_ref()
        .map(|action| action.target.as_str()),
      Some("notepad")
    );
  }

  #[test]
  fn rejects_unknown_request_honestly() {
    let plan = plan_user_request("delete all files".to_string());
    assert!(!plan.can_execute_directly);
    assert_eq!(plan.risk_level, "not-implemented");
    assert!(plan.suggested_action.is_none());
  }

  #[test]
  fn plans_parameterized_site_request() {
    let plan = plan_user_request("open site openai.com".to_string());
    assert!(plan.can_execute_directly);
    assert_eq!(
      plan
        .suggested_action
        .as_ref()
        .map(|action| action.target.as_str()),
      Some("https://openai.com")
    );
  }

  #[test]
  fn plans_web_search_request() {
    let plan = plan_user_request("search web tauri tray icon".to_string());
    assert!(plan.can_execute_directly);
    assert_eq!(
      plan.suggested_action.as_ref().map(|action| action.kind.as_str()),
      Some("search_web")
    );
  }

  #[test]
  fn plans_parameterized_folder_request() {
    let plan = plan_user_request("open folder downloads".to_string());
    assert!(plan.can_execute_directly);
    assert_eq!(
      plan.suggested_action.as_ref().map(|action| action.kind.as_str()),
      Some("open_folder")
    );
  }

  #[test]
  fn plans_parameterized_app_request() {
    let plan = plan_user_request("open app calculator".to_string());
    assert!(plan.can_execute_directly);
    assert_eq!(
      plan
        .suggested_action
        .as_ref()
        .map(|action| action.target.as_str()),
      Some("calculator")
    );
  }

  #[test]
  fn plans_music_player_request() {
    let plan = plan_user_request("open music player".to_string());
    assert!(plan.can_execute_directly);
    assert_eq!(
      plan.suggested_action.as_ref().map(|action| action.kind.as_str()),
      Some("open_app")
    );
    assert_eq!(
      plan
        .suggested_action
        .as_ref()
        .map(|action| action.target.as_str()),
      Some("music")
    );
  }

  #[test]
  fn plans_type_text_request_with_high_risk_script() {
    let plan = plan_user_request("type hello from xixi".to_string());
    assert!(plan.can_execute_directly);
    assert_eq!(plan.risk_level, "high-risk");
    assert_eq!(
      plan.suggested_action.as_ref().map(|action| action.kind.as_str()),
      Some("run_script")
    );

    let payload: ScriptTargetPayload = serde_json::from_str(
      &plan
        .suggested_action
        .as_ref()
        .expect("action should exist")
        .target,
    )
    .expect("payload should parse");
    assert_eq!(payload.script, "safe_desktop_action.py");
    assert_eq!(payload.input, Some("type:hello from xixi".to_string()));
  }

  #[test]
  fn plans_watch_screen_request_with_default_payload() {
    let plan = plan_user_request("watch screen".to_string());
    assert!(plan.can_execute_directly);
    assert_eq!(plan.risk_level, "medium-risk");
    assert_eq!(
      plan.suggested_action.as_ref().map(|action| action.kind.as_str()),
      Some("run_script")
    );

    let payload: ScriptTargetPayload = serde_json::from_str(
      &plan
        .suggested_action
        .as_ref()
        .expect("action should exist")
        .target,
    )
    .expect("payload should parse");
    assert_eq!(payload.script, "screen_watch_ocr.py");
    assert_eq!(
      payload.input,
      Some("keyword=stock duration=20 interval=1 max_hits=2".to_string())
    );
  }

  #[test]
  fn plans_screen_intent_request_with_default_payload() {
    let plan = plan_user_request("screen intent".to_string());
    assert!(plan.can_execute_directly);
    assert_eq!(plan.risk_level, "medium-risk");
    assert_eq!(
      plan.suggested_action.as_ref().map(|action| action.kind.as_str()),
      Some("run_script")
    );

    let payload: ScriptTargetPayload = serde_json::from_str(
      &plan
        .suggested_action
        .as_ref()
        .expect("action should exist")
        .target,
    )
    .expect("payload should parse");
    assert_eq!(payload.script, "screen_intent_watch.py");
    assert_eq!(
      payload.input,
      Some("goal=desktop-workflow duration=18 interval=1.2 samples=8".to_string())
    );
  }

  #[test]
  fn plans_screen_intent_request_with_goal_hint() {
    let plan = plan_user_request("screen intent review code".to_string());
    assert!(plan.can_execute_directly);
    assert_eq!(plan.risk_level, "medium-risk");

    let payload: ScriptTargetPayload = serde_json::from_str(
      &plan
        .suggested_action
        .as_ref()
        .expect("action should exist")
        .target,
    )
    .expect("payload should parse");
    assert_eq!(payload.script, "screen_intent_watch.py");
    assert_eq!(
      payload.input,
      Some("goal=review_code duration=18 interval=1.2 samples=8".to_string())
    );
  }

  #[test]
  fn plans_screen_behavior_request_with_default_payload() {
    let plan = plan_user_request("watch screen behavior".to_string());
    assert!(plan.can_execute_directly);
    assert_eq!(plan.risk_level, "medium-risk");

    let payload: ScriptTargetPayload = serde_json::from_str(
      &plan
        .suggested_action
        .as_ref()
        .expect("action should exist")
        .target,
    )
    .expect("payload should parse");
    assert_eq!(payload.script, "screen_behavior_watch.py");
    assert_eq!(
      payload.input,
      Some("goal=workflow duration=16 interval=1.0 samples=10".to_string())
    );
  }

  #[test]
  fn plans_latest_screen_behavior_report_request() {
    let plan = plan_user_request("latest screen behavior".to_string());
    assert!(plan.can_execute_directly);
    assert_eq!(plan.risk_level, "low-risk");
    assert_eq!(
      plan.suggested_action.as_ref().map(|action| action.kind.as_str()),
      Some("read_behavior_report")
    );
    assert_eq!(
      plan
        .suggested_action
        .as_ref()
        .map(|action| action.target.as_str()),
      Some("screen_behavior_watch")
    );
  }

  #[test]
  fn plans_latest_screen_intent_report_request() {
    let plan = plan_user_request("latest screen intent".to_string());
    assert!(plan.can_execute_directly);
    assert_eq!(plan.risk_level, "low-risk");
    assert_eq!(
      plan.suggested_action.as_ref().map(|action| action.kind.as_str()),
      Some("read_intent_report")
    );
    assert_eq!(
      plan
        .suggested_action
        .as_ref()
        .map(|action| action.target.as_str()),
      Some("screen_intent_watch")
    );
  }

  #[test]
  fn plans_human_move_request_with_coordinates() {
    let plan = plan_user_request("human move 1000,620".to_string());
    assert!(plan.can_execute_directly);
    assert_eq!(plan.risk_level, "high-risk");
    assert_eq!(
      plan.suggested_action.as_ref().map(|action| action.kind.as_str()),
      Some("run_script")
    );

    let payload: ScriptTargetPayload = serde_json::from_str(
      &plan
        .suggested_action
        .as_ref()
        .expect("action should exist")
        .target,
    )
    .expect("payload should parse");
    assert_eq!(payload.script, "human_input_ops.py");
    assert_eq!(payload.input, Some("move:1000,620".to_string()));
  }

  #[test]
  fn plans_right_click_request_with_desktop_skill_ops() {
    let plan = plan_user_request("right click".to_string());
    assert!(plan.can_execute_directly);
    assert_eq!(plan.risk_level, "high-risk");
    assert_eq!(
      plan.suggested_action.as_ref().map(|action| action.kind.as_str()),
      Some("run_script")
    );

    let payload: ScriptTargetPayload = serde_json::from_str(
      &plan
        .suggested_action
        .as_ref()
        .expect("action should exist")
        .target,
    )
    .expect("payload should parse");
    assert_eq!(payload.script, "desktop_skill_ops.py");
    assert_eq!(payload.input, Some("rightclick".to_string()));
  }

  #[test]
  fn plans_move_mouse_request_with_coordinates() {
    let plan = plan_user_request("move mouse 1200,720".to_string());
    assert!(plan.can_execute_directly);
    assert_eq!(plan.risk_level, "high-risk");
    assert_eq!(
      plan.suggested_action.as_ref().map(|action| action.kind.as_str()),
      Some("run_script")
    );

    let payload: ScriptTargetPayload = serde_json::from_str(
      &plan
        .suggested_action
        .as_ref()
        .expect("action should exist")
        .target,
    )
    .expect("payload should parse");
    assert_eq!(payload.script, "desktop_skill_ops.py");
    assert_eq!(payload.input, Some("move:1200,720".to_string()));
  }

  #[test]
  fn resolves_firefox_alias() {
    let alias = resolve_app_alias("firefox");
    assert_eq!(alias, Some(("firefox", "Mozilla Firefox")));
  }

  #[test]
  fn resolves_vscode_alias() {
    let alias = resolve_app_alias("vscode");
    assert_eq!(alias, Some(("vscode", "Visual Studio Code")));
  }

  #[test]
  fn resolves_terminal_alias() {
    let alias = resolve_app_alias("terminal");
    assert_eq!(alias, Some(("powershell", "PowerShell")));
  }

  #[test]
  fn parses_skill_command_with_input() {
    let parsed = extract_skill_command("run skill search_stock_news tsla");
    assert_eq!(
      parsed,
      Some(("search_stock_news".to_string(), Some("tsla".to_string())))
    );
  }

  #[test]
  fn renders_skill_template_with_input() {
    let skill = LocalSkillDefinition {
      id: "k1".into(),
      name: "Search".into(),
      description: "".into(),
      kind: "search_web".into(),
      target_template: "{{input}} stock news".into(),
      label_template: Some("news {{input}}".into()),
      risk_level: Some("low-risk".into()),
      aliases: None,
    };

    let action = render_skill_action(&skill, "nvda").expect("skill should render");
    assert_eq!(action.kind, "search_web");
    assert!(action.target.contains("nvda+stock+news"));
    assert_eq!(action.label, "news nvda");
  }

  #[test]
  fn renders_run_script_skill_payload() {
    let skill = LocalSkillDefinition {
      id: "screen_watch_stub".into(),
      name: "Screen Watch Stub".into(),
      description: "".into(),
      kind: "run_script".into(),
      target_template: "screen_watch_ocr.py".into(),
      label_template: Some("Screen Watch Stub".into()),
      risk_level: Some("medium-risk".into()),
      aliases: None,
    };

    let action = render_skill_action(&skill, "stock").expect("skill should render");
    assert_eq!(action.kind, "run_script");

    let payload: ScriptTargetPayload =
      serde_json::from_str(&action.target).expect("payload must be valid json");
    assert_eq!(payload.script, "screen_watch_ocr.py");
    assert_eq!(payload.input, Some("stock".to_string()));
  }

  #[test]
  fn sanitizes_path_token_for_log_filename() {
    assert_eq!(sanitize_path_token("screen watch/ocr"), "screen_watch_ocr");
    assert_eq!(sanitize_path_token(""), "skill_run");
  }

  #[test]
  fn builds_chat_endpoint_from_base_url() {
    assert_eq!(
      build_chat_completions_endpoint("https://api.openai.com/v1"),
      "https://api.openai.com/v1/chat/completions"
    );
    assert_eq!(
      build_chat_completions_endpoint("https://api.openai.com"),
      "https://api.openai.com/v1/chat/completions"
    );
    assert_eq!(
      build_chat_completions_endpoint("https://example.com/v1/chat/completions"),
      "https://example.com/v1/chat/completions"
    );
  }

  #[test]
  fn detects_html_upstream_error_shape() {
    let html = "<html><head><title>400 Bad Request</title></head><body>bad</body></html>";
    assert!(looks_like_html_response(html));
    assert_eq!(extract_html_title(html), Some("400 Bad Request".to_string()));
  }

  #[test]
  fn extracts_model_content_from_chat_completions() {
    let payload = json!({
      "choices": [{
        "message": {
          "content": "hello from model"
        }
      }]
    });

    let content = extract_model_content(&payload).expect("content should exist");
    assert_eq!(content, "hello from model");
  }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .manage(AppRuntimeState::default())
    .setup(|app| {
      if cfg!(debug_assertions) {
        app.handle().plugin(
          tauri_plugin_log::Builder::default()
            .level(log::LevelFilter::Info)
            .build(),
        )?;
      }

      if let Err(error) = ensure_skills_dir() {
        eprintln!("failed to initialize local skills folder: {error}");
      }
      if let Err(error) = ensure_pet_window(app.handle()) {
        eprintln!("failed to initialize pet window: {error}");
      }

      let show_item = MenuItem::with_id(app, "tray_show", "Show / Restore xixi", true, None::<&str>)?;
      let hide_item = MenuItem::with_id(app, "tray_hide", "Hide to tray", true, None::<&str>)?;
      let quit_item = MenuItem::with_id(app, "tray_quit", "Quit xixi", true, None::<&str>)?;
      let tray_menu = Menu::with_items(app, &[&show_item, &hide_item, &quit_item])?;

      let mut tray_builder = TrayIconBuilder::with_id("xixi-tray")
        .tooltip("xixi is running")
        .menu(&tray_menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
          "tray_show" => show_main_window(app),
          "tray_hide" => hide_main_window(app),
          "tray_quit" => {
            app
              .state::<AppRuntimeState>()
              .quitting
              .store(true, Ordering::Relaxed);
            app.exit(0);
          }
          _ => {}
        })
        .on_tray_icon_event(|tray, event| {
          if let TrayIconEvent::Click {
            button: MouseButton::Left,
            button_state: MouseButtonState::Up,
            ..
          } = event
          {
            let app = tray.app_handle();
            toggle_main_window(&app);
          }
        });

      if let Some(icon) = app.default_window_icon().cloned() {
        tray_builder = tray_builder.icon(icon);
      }

      tray_builder.build(app)?;
      Ok(())
    })
    .on_window_event(|window, event| {
      if let WindowEvent::CloseRequested { api, .. } = event {
        let quitting = window
          .state::<AppRuntimeState>()
          .quitting
          .load(Ordering::Relaxed);
        if !quitting && window.label() == "main" {
          api.prevent_close();
          hide_main_window(&window.app_handle());
        } else if !quitting && window.label() == "pet" {
          api.prevent_close();
          hide_pet_window(&window.app_handle());
        }
      }
    })
    .invoke_handler(tauri::generate_handler![
      get_desktop_profile,
      list_local_skills,
      get_skills_folder_path,
      chat_with_model_api,
      plan_user_request,
      execute_local_action,
      minimize_to_pet,
      restore_main_from_pet,
      quit_application
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
