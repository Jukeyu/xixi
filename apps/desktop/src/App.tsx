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
      'Supported right now: open QMDownload, open xixi folder, open GitHub, open weather page, open Chrome, open Edge, open Notepad, open Explorer.',
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
]

const quickActions = [
  'Open QMDownload',
  'Open xixi folder',
  'Open GitHub',
  'Open Chrome',
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

function App() {
  const isDesktop = '__TAURI_INTERNALS__' in window
  const runtimeMode = isDesktop ? 'Desktop shell' : 'Browser preview'
  const [desktopProfile, setDesktopProfile] = useState<DesktopProfile | null>(null)
  const [messages, setMessages] = useState(initialMessages)
  const [actionQueue, setActionQueue] = useState(initialQueue)
  const [draft, setDraft] = useState('Open GitHub')
  const [isBusy, setIsBusy] = useState(false)
  const [isMaximized, setIsMaximized] = useState(true)
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
    document.documentElement.dataset.theme = settings.theme
    document.documentElement.style.setProperty(
      '--ui-font-scale',
      settings.fontScale.toString()
    )
    window.localStorage.setItem(SETTINGS_STORAGE_KEY, JSON.stringify(settings))
  }, [settings])

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

      if (isDesktop && settings.autoRunSupported && plan.can_execute_directly && plan.suggested_action) {
        setActionQueue((current) =>
          current.map((item, index) =>
            index === 0 ? { ...item, state: 'running' } : item
          )
        )

        const result = await invoke<ActionExecutionResult>('execute_local_action', {
          action: plan.suggested_action,
        })

        setActionQueue((current) =>
          current.map((item) => ({
            ...item,
            state: result.ok ? 'done' : item.state === 'running' ? 'error' : item.state,
          }))
        )

        appendAssistantMessage(
          result.summary,
          result.details.length > 0 ? result.details.join(' / ') : 'Desktop action completed'
        )
      } else if (!settings.autoRunSupported && plan.can_execute_directly) {
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

  const resetConversation = () => {
    setMessages(initialMessages)
    setActionQueue(initialQueue)
  }

  const toggleTheme = () => {
    setSettings((current) => ({
      ...current,
      theme: current.theme === 'light' ? 'dark' : 'light',
    }))
  }

  const refreshWeather = () => {
    setSettings((current) => ({ ...current }))
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
            The contrast, theme, settings, and window controls in this build are all real. What you see here is meant to be usable, not decorative filler.
          </p>
          <div className="pet-card__status">
            <span className="status-dot" />
            {isBusy ? 'Busy: planning and running a real action' : 'Idle: waiting for a supported command'}
          </div>
          <div className="self-talk">
            {isBusy
              ? '"I am working through a real action path now."'
              : '"You can right-click anywhere to open the settings menu."'}
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
      </aside>

      <main className={`workspace ${settings.compactMode ? 'is-compact' : ''}`}>
        <header className="workspace-header">
          <div>
            <span className="eyebrow">Phase 1 Workspace</span>
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
              <button type="button" onClick={() => void handleClose()} title="Close">
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
                Right now I only execute commands that are actually wired. Everything else stays explicit.
              </div>
              <div className="composer__row">
                <textarea
                  value={draft}
                  onChange={(event) => setDraft(event.target.value)}
                  placeholder="Type a supported command"
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
            </ul>
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
                max="1.2"
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
          <button type="button" onClick={() => setSettingsOpen(true)}>
            Open settings
          </button>
          <button type="button" onClick={resetConversation}>
            Reset chat
          </button>
          <button type="button" onClick={refreshWeather}>
            Refresh weather
          </button>
        </div>
      ) : null}
    </div>
  )
}

export default App
