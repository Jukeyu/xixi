use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
  env,
  fs::{self, OpenOptions},
  io::Write,
  path::{Path, PathBuf},
  process::Command,
  sync::atomic::{AtomicBool, Ordering},
  time::{Instant, SystemTime, UNIX_EPOCH},
};
use tauri::{
  menu::{Menu, MenuItem},
  tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
  Manager, WindowEvent,
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
      kind: skill.kind,
      risk_level: skill.risk_level.unwrap_or_else(|| "low-risk".into()),
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
      "Try one of: chrome, edge, notepad, explorer, calculator, paint.",
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
    risk_level: skill.risk_level.clone().unwrap_or_else(|| "low-risk".into()),
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

  for skill in default_local_skills() {
    let path = dir.join(format!("{}.json", skill.id));
    if path.exists() {
      continue;
    }
    let content = serde_json::to_string_pretty(&skill)
      .map_err(|error| format!("Failed to serialize default skill: {error}"))?;
    fs::write(&path, content).map_err(|error| format!("Failed to write default skill: {error}"))?;
  }

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

fn try_spawn_any(candidates: &[&str]) -> bool {
  candidates
    .iter()
    .any(|candidate| Command::new(candidate).spawn().is_ok())
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

fn resolve_app_alias(query: &str) -> Option<(&'static str, &'static str)> {
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

fn recovery_tips_for_action(action: &LocalAction) -> Vec<String> {
  match action.kind.as_str() {
    "open_app" => vec![
      "Check whether the target app is installed on this Windows machine.".into(),
      "Try a different app alias like: chrome, edge, notepad, explorer, calculator, paint.".into(),
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

fn show_main_window<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
  if let Some(window) = app.get_webview_window("main") {
    let _ = window.show();
    let _ = window.unminimize();
    let _ = window.set_focus();
  }
}

fn hide_main_window<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
  if let Some(window) = app.get_webview_window("main") {
    let _ = window.hide();
  }
}

fn toggle_main_window<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
  if let Some(window) = app.get_webview_window("main") {
    if window.is_visible().unwrap_or(false) {
      let _ = window.hide();
    } else {
      let _ = window.show();
      let _ = window.unminimize();
      let _ = window.set_focus();
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
        if !quitting {
          api.prevent_close();
          let _ = window.hide();
        }
      }
    })
    .invoke_handler(tauri::generate_handler![
      get_desktop_profile,
      list_local_skills,
      get_skills_folder_path,
      plan_user_request,
      execute_local_action,
      quit_application
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
