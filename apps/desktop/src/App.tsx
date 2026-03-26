import { startTransition, useEffect, useMemo, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'
import './App.css'

type ChatMessage = {
  id: string
  role: 'assistant' | 'user'
  author: string
  content: string
  meta?: string
}

type ActionItem = {
  id: string
  title: string
  detail: string
  state: 'ready' | 'running' | 'waiting' | 'done' | 'error'
}

type DesktopProfile = {
  app_name: string
  runtime: string
  action_mode: string
  notes: string[]
}

type LocalAction = {
  kind: string
  target: string
  label: string
}

type CommandPlan = {
  assistant_reply: string
  risk_level: string
  can_execute_directly: boolean
  steps: ActionItem[]
  suggested_action?: LocalAction | null
}

type ActionExecutionResult = {
  ok: boolean
  summary: string
  details: string[]
  action_id: string
  duration_ms: number
  executed_at_ms: number
  recovery_tips: string[]
}

type ActionLogEntry = {
  id: string
  request: string
  actionLabel: string
  actionKind: string
  actionTarget: string
  ok: boolean
  summary: string
  details: string[]
  recoveryTips: string[]
  durationMs: number
  executedAtMs: number
}

type LocalSkillSummary = {
  id: string
  name: string
  description: string
  kind: string
  risk_level: string
  aliases: string[]
}

type ThemeMode = 'light' | 'dark'

type SettingsState = {
  theme: ThemeMode
  fontScale: number
  compactMode: boolean
  autoRunSupported: boolean
  weatherLocationName: string
  weatherLatitude: number
  weatherLongitude: number
}

type WeatherState = {
  loading: boolean
  summary: string
  temperatureText: string
  updatedAt: string
  error?: string
}

type ContextMenuState = {
  open: boolean
  x: number
  y: number
}

const SETTINGS_STORAGE_KEY = 'xixi.desktop.settings.v1'
const ACTION_LOG_STORAGE_KEY = 'xixi.desktop.action-log.v1'
const MAX_ACTION_LOGS = 60

const defaultSettings: SettingsState = {
  theme: 'light',
  fontScale: 1,
  compactMode: false,
  autoRunSupported: true,
  weatherLocationName: 'Taipei',
  weatherLatitude: 25.033,
  weatherLongitude: 121.5654,
}

const initialMessages: ChatMessage[] = [
  {
    id: 'm1',
    role: 'assistant',
    author: 'xixi',
    content:
      'This build only surfaces real features. The window buttons, weather card, theme toggle, and supported desktop actions are all wired to working code.',
    meta: 'No fake execution. No pretend widgets.',
  },
  {
    id: 'm2',
    role: 'assistant',
    author: 'xixi',
    content:
      'Supported now includes parameterized commands: open site openai.com, search web tauri tray icon, open folder downloads, open app calculator.',
    meta: 'Unsupported requests stay explicit.',
  },
]

const initialQueue: ActionItem[] = [
  {
    id: 'boot-1',
    title: 'Window controls',
    detail: 'Minimize, maximize, and close buttons use the Tauri desktop window',
    state: 'done',
  },
  {
    id: 'boot-2',
    title: 'Settings',
    detail: 'Theme and display settings persist locally in the app',
    state: 'done',
  },
  {
    id: 'boot-3',
    title: 'Live weather',
    detail: 'Weather card fetches real data from Open-Meteo',
    state: 'done',
  },
  {
    id: 'boot-4',
    title: 'Tray + logs',
    detail: 'Close hides to tray and each action writes structured logs',
    state: 'done',
  },
]

const quickActions = [
  'Open folder downloads',
  'Open site github.com',
  'Search web Tauri tray icon',
  'Run skill screen_watch_ocr keyword=stock duration=15',
  'Run skill desktop_action_safe click',
  'Open app calculator',
  'Open xixi folder',
]

const supportedCommands = [
  'Open QMDownload',
  'Open xixi folder',
  'Open GitHub',
  'Open weather',
  'Open Chrome',
  'Open Edge',
  'Open Notepad',
  'Open Explorer',
  'Open app calculator',
  'Open app paint',
  'Open folder downloads',
  'Open folder desktop',
  'Open site <domain>',
  'Search web <query>',
  'Run skill <id> [input]',
  'Open skills folder',
]

function makeId(prefix: string) {
  return `${prefix}-${crypto.randomUUID()}`
}

function readStoredSettings(): SettingsState {
  if (typeof window === 'undefined') {
    return defaultSettings
  }

  try {
    const raw = window.localStorage.getItem(SETTINGS_STORAGE_KEY)
    if (!raw) {
      return defaultSettings
    }

    return { ...defaultSettings, ...JSON.parse(raw) }
  } catch {
    return defaultSettings
  }
}

function readStoredActionLogs(): ActionLogEntry[] {
  if (typeof window === 'undefined') {
    return []
  }

  try {
    const raw = window.localStorage.getItem(ACTION_LOG_STORAGE_KEY)
    if (!raw) {
      return []
    }

    const parsed = JSON.parse(raw) as ActionLogEntry[]
    if (!Array.isArray(parsed)) {
      return []
    }

    return parsed.slice(0, MAX_ACTION_LOGS)
  } catch {
    return []
  }
}

function clampFontScale(value: number) {
  return Math.max(0.9, Math.min(1.4, Number(value.toFixed(2))))
}

function weatherCodeToText(code: number) {
  if (code === 0) return 'Clear'
  if ([1, 2, 3].includes(code)) return 'Cloudy'
  if ([45, 48].includes(code)) return 'Fog'
  if ([51, 53, 55, 56, 57].includes(code)) return 'Drizzle'
  if ([61, 63, 65, 66, 67, 80, 81, 82].includes(code)) return 'Rain'
  if ([71, 73, 75, 77, 85, 86].includes(code)) return 'Snow'
  if ([95, 96, 99].includes(code)) return 'Storm'
  return 'Unknown'
}

function formatTimestamp(timestampMs: number) {
  return new Date(timestampMs).toLocaleString()
}

function App() {
  const isDesktop = '__TAURI_INTERNALS__' in window
  const runtimeMode = isDesktop ? 'Desktop shell' : 'Browser preview'
  const [desktopProfile, setDesktopProfile] = useState<DesktopProfile | null>(null)
  const [messages, setMessages] = useState(initialMessages)
  const [actionQueue, setActionQueue] = useState(initialQueue)
  const [draft, setDraft] = useState('Open site github.com')
  const [isBusy, setIsBusy] = useState(false)
  const [isMaximized, setIsMaximized] = useState(false)
  const [settingsOpen, setSettingsOpen] = useState(false)
  const [settings, setSettings] = useState<SettingsState>(() => readStoredSettings())
  const [weather, setWeather] = useState<WeatherState>({
    loading: true,
    summary: 'Loading real weather data...',
    temperatureText: '--',
    updatedAt: '',
  })
  const [contextMenu, setContextMenu] = useState<ContextMenuState>({
    open: false,
    x: 0,
    y: 0,
  })
  const [actionLogs, setActionLogs] = useState<ActionLogEntry[]>(() => readStoredActionLogs())
  const [lastFailedAction, setLastFailedAction] = useState<LocalAction | null>(null)
  const [localSkills, setLocalSkills] = useState<LocalSkillSummary[]>([])
  const [skillsFolderPath, setSkillsFolderPath] = useState('')
  const [weatherReloadTick, setWeatherReloadTick] = useState(0)

  const windowApi = useMemo(() => (isDesktop ? getCurrentWindow() : null), [isDesktop])

  useEffect(() => {
    if (!isDesktop) {
      return
    }

    invoke<DesktopProfile>('get_desktop_profile')
      .then((profile) => setDesktopProfile(profile))
      .catch(() => setDesktopProfile(null))
  }, [isDesktop])

  useEffect(() => {
    if (!isDesktop) {
      return
    }

    invoke<LocalSkillSummary[]>('list_local_skills')
      .then((skills) => setLocalSkills(skills))
      .catch(() => setLocalSkills([]))

    invoke<string>('get_skills_folder_path')
      .then((path) => setSkillsFolderPath(path))
      .catch(() => setSkillsFolderPath(''))
  }, [isDesktop])

  useEffect(() => {
    if (!windowApi) {
      return
    }

    windowApi
      .isMaximized()
      .then((value) => setIsMaximized(value))
      .catch(() => setIsMaximized(false))
  }, [windowApi])

  useEffect(() => {
    document.documentElement.dataset.theme = settings.theme
    document.documentElement.style.setProperty(
      '--ui-font-scale',
      settings.fontScale.toString()
    )
    window.localStorage.setItem(SETTINGS_STORAGE_KEY, JSON.stringify(settings))
  }, [settings])

  useEffect(() => {
    window.localStorage.setItem(
      ACTION_LOG_STORAGE_KEY,
      JSON.stringify(actionLogs.slice(0, MAX_ACTION_LOGS))
    )
  }, [actionLogs])

  useEffect(() => {
    let cancelled = false

    async function loadWeather() {
      setWeather((current) => ({
        ...current,
        loading: true,
        error: undefined,
      }))

      try {
        const params = new URLSearchParams({
          latitude: String(settings.weatherLatitude),
          longitude: String(settings.weatherLongitude),
          current: 'temperature_2m,wind_speed_10m,weather_code',
          timezone: 'auto',
        })
        const response = await fetch(
          `https://api.open-meteo.com/v1/forecast?${params.toString()}`
        )

        if (!response.ok) {
          throw new Error(`Weather request failed with ${response.status}`)
        }

        const data = (await response.json()) as {
          current?: {
            temperature_2m: number
            wind_speed_10m: number
            weather_code: number
            time: string
          }
        }

        if (!data.current) {
          throw new Error('Weather payload did not include current data')
        }

        if (cancelled) {
          return
        }

        const weatherLabel = weatherCodeToText(data.current.weather_code)
        setWeather({
          loading: false,
          summary: `${weatherLabel} in ${settings.weatherLocationName}`,
          temperatureText: `${Math.round(data.current.temperature_2m)}°C`,
          updatedAt: new Date(data.current.time).toLocaleString(),
        })
      } catch (error) {
        if (cancelled) {
          return
        }

        setWeather({
          loading: false,
          summary: 'Weather data unavailable',
          temperatureText: '--',
          updatedAt: '',
          error: error instanceof Error ? error.message : 'Unknown weather error',
        })
      }
    }

    void loadWeather()

    return () => {
      cancelled = true
    }
  }, [
    settings.weatherLatitude,
    settings.weatherLongitude,
    settings.weatherLocationName,
    weatherReloadTick,
  ])

  useEffect(() => {
    const closeMenu = () => setContextMenu((current) => ({ ...current, open: false }))
    window.addEventListener('click', closeMenu)
    return () => window.removeEventListener('click', closeMenu)
  }, [])

  const appendAssistantMessage = (content: string, meta?: string) => {
    startTransition(() => {
      setMessages((current) => [
        ...current,
        {
          id: makeId('assistant'),
          role: 'assistant',
          author: 'xixi',
          content,
          meta,
        },
      ])
    })
  }

  const appendActionLog = (
    requestText: string,
    action: LocalAction,
    result: ActionExecutionResult
  ) => {
    const nextEntry: ActionLogEntry = {
      id: result.action_id,
      request: requestText,
      actionLabel: action.label,
      actionKind: action.kind,
      actionTarget: action.target,
      ok: result.ok,
      summary: result.summary,
      details: result.details,
      recoveryTips: result.recovery_tips,
      durationMs: result.duration_ms,
      executedAtMs: result.executed_at_ms,
    }

    setActionLogs((current) => [nextEntry, ...current].slice(0, MAX_ACTION_LOGS))
  }

  const executePlannedAction = async (action: LocalAction, requestText: string) => {
    setActionQueue((current) =>
      current.map((item, index) =>
        index === 0 ? { ...item, state: 'running' } : item
      )
    )

    const result = await invoke<ActionExecutionResult>('execute_local_action', {
      action,
    })

    appendActionLog(requestText, action, result)

    setActionQueue((current) =>
      current.map((item) =>
        item.state === 'running'
          ? { ...item, state: result.ok ? 'done' : 'error' }
          : item
      )
    )

    appendAssistantMessage(
      result.summary,
      `${result.ok ? 'success' : 'failed'} | ${result.duration_ms} ms | ${result.action_id}`
    )

    if (result.details.length > 0) {
      appendAssistantMessage('Action details', result.details.join(' / '))
    }

    if (!result.ok) {
      setLastFailedAction(action)
      if (result.recovery_tips.length > 0) {
        appendAssistantMessage('Recovery suggestions', result.recovery_tips.join(' / '))
      }
    } else {
      setLastFailedAction(null)
    }

    return result
  }

  const runRequest = async (request: string) => {
    const trimmed = request.trim()
    if (!trimmed || isBusy) {
      return
    }

    setIsBusy(true)
    setDraft('')

    startTransition(() => {
      setMessages((current) => [
        ...current,
        {
          id: makeId('user'),
          role: 'user',
          author: 'You',
          content: trimmed,
        },
      ])
    })

    try {
      const plan = await invoke<CommandPlan>('plan_user_request', {
        request: trimmed,
      })

      setActionQueue(plan.steps)
      appendAssistantMessage(
        plan.assistant_reply,
        `${plan.risk_level} | ${plan.can_execute_directly ? 'real action available' : 'not implemented'}`
      )

      if (
        isDesktop &&
        settings.autoRunSupported &&
        plan.can_execute_directly &&
        plan.suggested_action
      ) {
        await executePlannedAction(plan.suggested_action, trimmed)
      } else if (!settings.autoRunSupported && plan.can_execute_directly) {
        if (plan.suggested_action) {
          setLastFailedAction(plan.suggested_action)
        }
        appendAssistantMessage(
          'The command is supported, but auto-run is off in settings. Turn it on to execute immediately.',
          'Manual safety mode is active'
        )
      } else if (!isDesktop && plan.can_execute_directly) {
        appendAssistantMessage(
          'This preview can show the plan, but only the desktop app can execute real system actions.',
          'Browser preview does not touch your desktop'
        )
      }
    } catch (error) {
      const detail = error instanceof Error ? error.message : 'Unknown error'
      appendAssistantMessage(
        'The action failed. I kept the failure visible instead of pretending it worked.',
        detail
      )
      setActionQueue((current) =>
        current.map((item, index) =>
          index === 0 ? { ...item, state: 'error' } : item
        )
      )
    } finally {
      setIsBusy(false)
    }
  }

  const retryLastFailedAction = async () => {
    if (!lastFailedAction || isBusy) {
      return
    }

    setIsBusy(true)
    try {
      appendAssistantMessage(
        'Retrying the last failed action.',
        `${lastFailedAction.kind} -> ${lastFailedAction.target}`
      )

      setActionQueue([
        {
          id: makeId('retry'),
          title: `Retry ${lastFailedAction.label}`,
          detail: `Retrying ${lastFailedAction.kind} (${lastFailedAction.target})`,
          state: 'ready',
        },
      ])

      await executePlannedAction(lastFailedAction, `Retry ${lastFailedAction.label}`)
    } catch (error) {
      const detail = error instanceof Error ? error.message : 'Unknown retry error'
      appendAssistantMessage('Retry failed again.', detail)
      setActionQueue((current) =>
        current.map((item, index) =>
          index === 0 ? { ...item, state: 'error' } : item
        )
      )
    } finally {
      setIsBusy(false)
    }
  }

  const resetConversation = () => {
    setMessages(initialMessages)
    setActionQueue(initialQueue)
    setLastFailedAction(null)
  }

  const toggleTheme = () => {
    setSettings((current) => ({
      ...current,
      theme: current.theme === 'light' ? 'dark' : 'light',
    }))
  }

  const refreshWeather = () => {
    setWeatherReloadTick((value) => value + 1)
  }

  const adjustFontScale = (delta: number) => {
    setSettings((current) => ({
      ...current,
      fontScale: clampFontScale(current.fontScale + delta),
    }))
  }

  const exportActionLogs = () => {
    if (actionLogs.length === 0) {
      appendAssistantMessage('No action logs to export yet.')
      return
    }

    const blob = new Blob([JSON.stringify(actionLogs, null, 2)], {
      type: 'application/json',
    })
    const link = document.createElement('a')
    const url = URL.createObjectURL(blob)
    link.href = url
    link.download = `xixi-action-log-${Date.now()}.json`
    document.body.append(link)
    link.click()
    link.remove()
    URL.revokeObjectURL(url)
  }

  const clearActionLogs = () => {
    setActionLogs([])
  }

  const refreshLocalSkills = async () => {
    if (!isDesktop) {
      return
    }
    try {
      const skills = await invoke<LocalSkillSummary[]>('list_local_skills')
      setLocalSkills(skills)
      appendAssistantMessage('Local skills refreshed.', `${skills.length} skill(s) loaded`)
    } catch (error) {
      const detail = error instanceof Error ? error.message : 'Unknown skills refresh error'
      appendAssistantMessage('Failed to refresh local skills.', detail)
    }
  }

  const runSkillById = async (skillId: string) => {
    await runRequest(`run skill ${skillId}`)
  }

  const onContextMenu: React.MouseEventHandler<HTMLDivElement> = (event) => {
    event.preventDefault()
    setContextMenu({
      open: true,
      x: event.clientX,
      y: event.clientY,
    })
  }

  const handleMinimize = async () => {
    if (!windowApi) return
    await windowApi.minimize()
  }

  const handleToggleMaximize = async () => {
    if (!windowApi) return
    if (isMaximized) {
      await windowApi.unmaximize()
      setIsMaximized(false)
    } else {
      await windowApi.maximize()
      setIsMaximized(true)
    }
  }

  const handleClose = async () => {
    if (!windowApi) return
    await windowApi.close()
  }

  const handleQuit = async () => {
    if (!isDesktop) return
    await invoke('quit_application')
  }

  return (
    <div className="app-shell" onContextMenu={onContextMenu}>
      <aside className="pet-panel">
        <div className="pet-card">
          <div className="pet-card__top">
            <span className="pet-card__badge">xixi</span>
            <span className="pet-card__runtime">{runtimeMode}</span>
          </div>
          <div className="pet-avatar" aria-hidden="true">
            <div className="pet-avatar__ear pet-avatar__ear--left" />
            <div className="pet-avatar__ear pet-avatar__ear--right" />
            <div className="pet-avatar__face">
              <span className="pet-avatar__eye" />
              <span className="pet-avatar__eye" />
              <span className="pet-avatar__nose" />
            </div>
          </div>
          <h1>Sharper, usable desktop UI</h1>
          <p className="pet-card__summary">
            The contrast, theme, settings, and window controls in this build are all real. What you
            see here is meant to be usable, not decorative filler.
          </p>
          <div className="pet-card__status">
            <span className="status-dot" />
            {isBusy ? 'Busy: planning and running a real action' : 'Idle: waiting for a supported command'}
          </div>
          <div className="self-talk">
            {isBusy
              ? '"I am working through a real action path now."'
              : '"You can right-click anywhere to open quick tuning controls."'}
          </div>
          {desktopProfile ? (
            <div className="runtime-note">
              <strong>Runtime status</strong>
              <ul>
                {desktopProfile.notes.map((note) => (
                  <li key={note}>{note}</li>
                ))}
              </ul>
            </div>
          ) : null}
        </div>

        <section className="weather-card">
          <div className="section-title">Live weather</div>
          <div className="weather-card__value">{weather.temperatureText}</div>
          <div className="weather-card__summary">{weather.summary}</div>
          <div className="weather-card__meta">
            {weather.loading
              ? 'Refreshing...'
              : weather.updatedAt
                ? `Updated ${weather.updatedAt}`
                : weather.error ?? 'No weather timestamp'}
          </div>
          <button type="button" className="ghost-button" onClick={refreshWeather}>
            Refresh weather
          </button>
        </section>

        <section className="sidebar-section">
          <div className="section-title">Supported now</div>
          <div className="supported-list">
            {supportedCommands.map((command) => (
              <div key={command} className="supported-item">
                {command}
              </div>
            ))}
          </div>
        </section>

        <section className="sidebar-section">
          <div className="section-title">Local skills</div>
          <div className="action-tools">
            <button type="button" className="ghost-button" onClick={() => void refreshLocalSkills()}>
              Refresh
            </button>
            <button type="button" className="ghost-button" onClick={() => void runRequest('open skills folder')}>
              Open folder
            </button>
          </div>
          {skillsFolderPath ? (
            <div className="skill-folder-path">{skillsFolderPath}</div>
          ) : null}
          <div className="skill-list">
            {localSkills.length === 0 ? (
              <article className="skill-card">
                <strong>No local skills loaded</strong>
                <p>Add skill JSON files in your local skills folder, then click Refresh.</p>
              </article>
            ) : (
              localSkills.slice(0, 6).map((skill) => (
                <article key={skill.id} className="skill-card">
                  <div className="skill-card__head">
                    <strong>{skill.name}</strong>
                    <span>{skill.risk_level}</span>
                  </div>
                  <p>{skill.description}</p>
                  <div className="skill-card__meta">{skill.id}</div>
                  <button type="button" className="ghost-button" onClick={() => void runSkillById(skill.id)}>
                    Run skill
                  </button>
                </article>
              ))
            )}
          </div>
        </section>
      </aside>

      <main className={`workspace ${settings.compactMode ? 'is-compact' : ''}`}>
        <header className="workspace-header">
          <div>
            <span className="eyebrow">Phase 2 Prototype Workspace</span>
            <h2>Readable and real</h2>
          </div>
          <div className="header-actions">
            <button type="button" className="ghost-button" onClick={toggleTheme}>
              {settings.theme === 'light' ? 'Dark mode' : 'Light mode'}
            </button>
            <button type="button" className="ghost-button" onClick={() => setSettingsOpen(true)}>
              Settings
            </button>
            <div className="window-actions" aria-label="Window controls">
              <button type="button" onClick={() => void handleMinimize()} title="Minimize">
                -
              </button>
              <button type="button" onClick={() => void handleToggleMaximize()} title="Toggle size">
                {isMaximized ? '[]' : '+'}
              </button>
              <button type="button" onClick={() => void handleClose()} title="Hide to tray">
                _
              </button>
              <button type="button" onClick={() => void handleQuit()} title="Quit">
                x
              </button>
            </div>
          </div>
        </header>

        <section className="quick-actions">
          {quickActions.map((action) => (
            <button key={action} type="button" onClick={() => void runRequest(action)}>
              {action}
            </button>
          ))}
        </section>

        <section className="chat-layout">
          <div className="chat-panel">
            <div className="chat-panel__scroll">
              {messages.map((message) => (
                <article
                  key={message.id}
                  className={`message-card message-card--${message.role}`}
                >
                  <div className="message-card__author">{message.author}</div>
                  <p>{message.content}</p>
                  {message.meta ? (
                    <div className="message-card__meta">{message.meta}</div>
                  ) : null}
                </article>
              ))}
            </div>

            <footer className="composer">
              <div className="composer__hint">
                I only execute commands that are actually wired. Try: run skill open_github
              </div>
              <div className="composer__row">
                <textarea
                  value={draft}
                  onChange={(event) => setDraft(event.target.value)}
                  placeholder="Try: run skill open_github / open site openai.com / open folder downloads"
                />
                <button
                  type="button"
                  onClick={() => void runRequest(draft)}
                  disabled={isBusy}
                >
                  {isBusy ? 'Running' : 'Send'}
                </button>
              </div>
            </footer>
          </div>

          <aside className="action-panel">
            <div className="section-title">Current plan</div>
            <div className="action-list">
              {actionQueue.map((item) => (
                <article key={item.id} className={`action-card is-${item.state}`}>
                  <div className="action-card__header">
                    <strong>{item.title}</strong>
                    <span>{item.state}</span>
                  </div>
                  <p>{item.detail}</p>
                </article>
              ))}
            </div>

            <div className="section-title">Reality rules</div>
            <ul className="goal-list">
              <li>No fake execution messages</li>
              <li>Theme and settings persist locally</li>
              <li>Weather card uses live API data</li>
              <li>Window controls call the real desktop shell</li>
              <li>Close hides to tray instead of silent exit</li>
              <li>Every execution writes a structured action log</li>
            </ul>

            {lastFailedAction ? (
              <div className="failed-action-box">
                <div className="section-title">Recovery</div>
                <p>
                  Last failed action: <strong>{lastFailedAction.label}</strong>
                </p>
                <button
                  type="button"
                  className="retry-button"
                  onClick={() => void retryLastFailedAction()}
                  disabled={isBusy}
                >
                  Retry last failed action
                </button>
              </div>
            ) : null}

            <div className="section-title">Action logs</div>
            <div className="action-tools">
              <button type="button" className="ghost-button" onClick={exportActionLogs}>
                Export JSON
              </button>
              <button type="button" className="ghost-button" onClick={clearActionLogs}>
                Clear
              </button>
            </div>

            <div className="log-list">
              {actionLogs.length === 0 ? (
                <article className="log-card">
                  <div className="log-card__head">
                    <strong>No logs yet</strong>
                  </div>
                  <p>Run a supported command to create structured entries.</p>
                </article>
              ) : (
                actionLogs.slice(0, 6).map((log) => (
                  <article
                    key={log.id}
                    className={`log-card ${log.ok ? 'is-ok' : 'is-fail'}`}
                  >
                    <div className="log-card__head">
                      <strong>{log.actionLabel}</strong>
                      <span>{log.ok ? 'ok' : 'failed'}</span>
                    </div>
                    <p>{log.summary}</p>
                    <div className="log-card__meta">
                      {log.durationMs} ms | {formatTimestamp(log.executedAtMs)}
                    </div>
                  </article>
                ))
              )}
            </div>
          </aside>
        </section>
      </main>

      {settingsOpen ? (
        <div className="settings-overlay" role="presentation" onClick={() => setSettingsOpen(false)}>
          <section
            className="settings-panel"
            role="dialog"
            aria-label="Settings"
            onClick={(event) => event.stopPropagation()}
          >
            <div className="settings-panel__header">
              <div>
                <div className="section-title">Settings</div>
                <h3>Adjust the live app</h3>
              </div>
              <button type="button" className="ghost-button" onClick={() => setSettingsOpen(false)}>
                Close
              </button>
            </div>

            <label className="settings-row">
              <span>Theme</span>
              <select
                value={settings.theme}
                onChange={(event) =>
                  setSettings((current) => ({
                    ...current,
                    theme: event.target.value as ThemeMode,
                  }))
                }
              >
                <option value="light">Light</option>
                <option value="dark">Dark</option>
              </select>
            </label>

            <label className="settings-row">
              <span>Font scale</span>
              <input
                type="range"
                min="0.9"
                max="1.4"
                step="0.05"
                value={settings.fontScale}
                onChange={(event) =>
                  setSettings((current) => ({
                    ...current,
                    fontScale: Number(event.target.value),
                  }))
                }
              />
            </label>

            <label className="settings-toggle">
              <input
                type="checkbox"
                checked={settings.compactMode}
                onChange={(event) =>
                  setSettings((current) => ({
                    ...current,
                    compactMode: event.target.checked,
                  }))
                }
              />
              <span>Compact layout</span>
            </label>

            <label className="settings-toggle">
              <input
                type="checkbox"
                checked={settings.autoRunSupported}
                onChange={(event) =>
                  setSettings((current) => ({
                    ...current,
                    autoRunSupported: event.target.checked,
                  }))
                }
              />
              <span>Auto-run supported commands</span>
            </label>

            <label className="settings-row">
              <span>Weather location name</span>
              <input
                type="text"
                value={settings.weatherLocationName}
                onChange={(event) =>
                  setSettings((current) => ({
                    ...current,
                    weatherLocationName: event.target.value,
                  }))
                }
              />
            </label>

            <label className="settings-row">
              <span>Latitude</span>
              <input
                type="number"
                step="0.0001"
                value={settings.weatherLatitude}
                onChange={(event) =>
                  setSettings((current) => ({
                    ...current,
                    weatherLatitude: Number(event.target.value),
                  }))
                }
              />
            </label>

            <label className="settings-row">
              <span>Longitude</span>
              <input
                type="number"
                step="0.0001"
                value={settings.weatherLongitude}
                onChange={(event) =>
                  setSettings((current) => ({
                    ...current,
                    weatherLongitude: Number(event.target.value),
                  }))
                }
              />
            </label>
          </section>
        </div>
      ) : null}

      {contextMenu.open ? (
        <div
          className="context-menu"
          style={{ left: contextMenu.x, top: contextMenu.y }}
          role="menu"
        >
          <button type="button" onClick={toggleTheme}>
            Toggle theme
          </button>
          <button type="button" onClick={() => adjustFontScale(0.05)}>
            Increase font
          </button>
          <button type="button" onClick={() => adjustFontScale(-0.05)}>
            Decrease font
          </button>
          <button
            type="button"
            onClick={() =>
              setSettings((current) => ({
                ...current,
                compactMode: !current.compactMode,
              }))
            }
          >
            Toggle compact mode
          </button>
          <button
            type="button"
            onClick={() =>
              setSettings((current) => ({
                ...current,
                autoRunSupported: !current.autoRunSupported,
              }))
            }
          >
            Toggle auto-run
          </button>
          <button type="button" onClick={() => setSettingsOpen(true)}>
            Open settings
          </button>
          <button type="button" onClick={resetConversation}>
            Reset chat
          </button>
          <button type="button" onClick={refreshWeather}>
            Refresh weather
          </button>
          <button type="button" onClick={() => void runRequest('open skills folder')}>
            Open skills folder
          </button>
          <button type="button" onClick={() => void refreshLocalSkills()}>
            Refresh skills
          </button>
          <button
            type="button"
            onClick={() => void retryLastFailedAction()}
            disabled={!lastFailedAction || isBusy}
          >
            Retry last failed
          </button>
        </div>
      ) : null}
    </div>
  )
}

export default App
