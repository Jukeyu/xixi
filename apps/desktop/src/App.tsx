import { startTransition, useEffect, useMemo, useRef, useState } from 'react'
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

type ModelChatApiRequest = {
  base_url: string
  api_key: string
  model: string
  user_prompt: string
  system_prompt?: string | null
  temperature?: number
  max_tokens?: number
}

type ModelChatApiResponse = {
  content: string
  model: string
  usage_summary?: string | null
  latency_ms: number
}

type BridgeRemoteCommand = {
  id: string
  source: string
  text: string
  received_at_ms: number
}

type PendingAction = {
  request: string
  riskLevel: string
  action: LocalAction
}

type FailedActionState = {
  request: string
  riskLevel: string
  action: LocalAction
}

type PermissionProfile = 'safe' | 'balanced' | 'advanced'

type ThemeMode = 'light' | 'dark'

type SettingsState = {
  theme: ThemeMode
  fontScale: number
  compactMode: boolean
  autoRunSupported: boolean
  permissionProfile: PermissionProfile
  chatMode: 'command' | 'model'
  modelBaseUrl: string
  modelName: string
  modelApiKey: string
  modelSystemPrompt: string
  modelTemperature: number
  modelMaxTokens: number
  weatherLocationName: string
  weatherLatitude: number
  weatherLongitude: number
  remoteBridgeEnabled: boolean
  remoteBridgePollSeconds: number
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
  permissionProfile: 'balanced',
  chatMode: 'command',
  modelBaseUrl: 'https://api.openai.com/v1',
  modelName: 'gpt-4o-mini',
  modelApiKey: '',
  modelSystemPrompt:
    'You are xixi, a practical desktop assistant. Be concise, honest, and action-oriented.',
  modelTemperature: 0.4,
  modelMaxTokens: 600,
  weatherLocationName: 'Taipei',
  weatherLatitude: 25.033,
  weatherLongitude: 121.5654,
  remoteBridgeEnabled: false,
  remoteBridgePollSeconds: 6,
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
      'Supported now includes parameterized commands: open site openai.com, search web tauri tray icon, open folder downloads, open app calculator, and screen intent observation.',
    meta: 'Unsupported requests stay explicit.',
  },
  {
    id: 'm3',
    role: 'assistant',
    author: 'xixi',
    content:
      'You can switch to Model Chat mode after filling API settings (base URL, model, key).',
    meta: 'Command mode and model mode can be switched anytime.',
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
  'Open app vscode',
  'Open terminal',
  'Search web Tauri tray icon',
  'Move mouse 960,540',
  'Click 960,540',
  'Double click 960,540',
  'Right click',
  'Right click 960,540',
  'Drag mouse 760,420 to 1120,640',
  'Scroll down 400',
  'Run skill screen_watch_ocr keyword=stock duration=15',
  'Latest screen watch',
  'Screen intent trading',
  'Run skill screen_intent_watch goal=trading duration=16 samples=6',
  'Latest screen intent',
  'Watch screen behavior workflow',
  'Latest screen behavior',
  'Desktop status',
  'Latest screen summary',
  'Page agent inspect example.com',
  'Page agent click example.com More information',
  'Latest page agent',
  'Human move 960,540',
  'Human click 920,520',
  'Run skill desktop_action_safe click',
  'Run skill desktop_skill_ops move:800,460',
  'Run skill human_input_ops drag:760,420>1080,640',
  'Type hello from xixi',
  'Hotkey ctrl,s',
  'Watch screen stock',
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
  'Open app firefox',
  'Open app vscode',
  'Open terminal',
  'Open Notepad',
  'Open Explorer',
  'Open music player',
  'Open app calculator',
  'Open app paint',
  'Open folder downloads',
  'Open folder desktop',
  'Open site <domain>',
  'Search web <query>',
  'Type <text>',
  'Press key <name>',
  'Hotkey <key1,key2>',
  'Move mouse <x,y>',
  'Click / Double click / Right click',
  'Click <x,y>',
  'Double click <x,y>',
  'Right click <x,y>',
  'Drag mouse <x1,y1> to <x2,y2>',
  'Scroll up|down <amount>',
  'Watch screen <keyword>',
  'Latest screen watch',
  'Screen intent [goal]',
  'Watch intent [goal]',
  'Latest screen intent',
  'Watch screen behavior [goal]',
  'Latest screen behavior',
  'Desktop status | Desktop brief',
  'Latest screen summary',
  'Page agent inspect <url>',
  'Page agent click <url> <text>',
  'Latest page agent',
  'Human move <x,y>',
  'Human click <x,y>',
  'Human drag <x1,y1> <x2,y2>',
  'Human type <text>',
  'Run skill <id> [input]',
  'Open skills folder',
]

const petPoses = ['sit', 'blink', 'stretch', 'play', 'sleep', 'jump', 'dance', 'listen'] as const
type PetPose = (typeof petPoses)[number]

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

function normalizeRiskLevel(riskLevel: string): 'low-risk' | 'medium-risk' | 'high-risk' | 'unknown' {
  if (riskLevel === 'low-risk' || riskLevel === 'medium-risk' || riskLevel === 'high-risk') {
    return riskLevel
  }
  return 'unknown'
}

function permissionProfileText(profile: PermissionProfile) {
  if (profile === 'safe') {
    return 'Safe: web/folder only'
  }
  if (profile === 'advanced') {
    return 'Advanced: includes high-risk scripts'
  }
  return 'Balanced: apps + scripts (high-risk blocked)'
}

function isActionAllowedByPermissionProfile(
  profile: PermissionProfile,
  action: LocalAction,
  riskLevel: string
) {
  const normalizedRisk = normalizeRiskLevel(riskLevel)

  if (profile === 'advanced') {
    return { allowed: true, reason: 'Advanced profile allows all current actions.' }
  }

  if (normalizedRisk === 'high-risk') {
    return {
      allowed: false,
      reason: `${profile} profile blocks high-risk actions.`,
    }
  }

  if (normalizedRisk === 'unknown') {
    return {
      allowed: false,
      reason: `${profile} profile blocked action with unknown risk level.`,
    }
  }

  if (profile === 'safe') {
    if (action.kind === 'open_url' || action.kind === 'search_web' || action.kind === 'open_folder') {
      return { allowed: true, reason: 'Allowed in Safe profile.' }
    }
    return {
      allowed: false,
      reason: 'Safe profile only allows open_url/search_web/open_folder.',
    }
  }

  return { allowed: true, reason: 'Allowed in Balanced profile.' }
}

function App() {
  const isDesktop = '__TAURI_INTERNALS__' in window
  const isPetWindow =
    typeof window !== 'undefined' &&
    new URLSearchParams(window.location.search).get('pet') === '1'
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
  const [lastFailedAction, setLastFailedAction] = useState<FailedActionState | null>(null)
  const [pendingAction, setPendingAction] = useState<PendingAction | null>(null)
  const [apiSetupOpen, setApiSetupOpen] = useState(false)
  const [petPose, setPetPose] = useState<PetPose>('sit')
  const [localSkills, setLocalSkills] = useState<LocalSkillSummary[]>([])
  const [skillsFolderPath, setSkillsFolderPath] = useState('')
  const [bridgeFolderPath, setBridgeFolderPath] = useState('')
  const [weatherReloadTick, setWeatherReloadTick] = useState(0)
  const isBusyRef = useRef(false)
  const runRequestRef = useRef<((request: string) => Promise<void>) | null>(null)

  const windowApi = useMemo(() => (isDesktop ? getCurrentWindow() : null), [isDesktop])
  const hasModelApiConfigured = useMemo(
    () =>
      settings.modelApiKey.trim().length > 0 &&
      settings.modelBaseUrl.trim().length > 0 &&
      settings.modelName.trim().length > 0,
    [settings.modelApiKey, settings.modelBaseUrl, settings.modelName]
  )
  const permissionText = useMemo(
    () => permissionProfileText(settings.permissionProfile),
    [settings.permissionProfile]
  )

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

    invoke<string>('get_bridge_folder_path')
      .then((path) => setBridgeFolderPath(path))
      .catch(() => setBridgeFolderPath(''))
  }, [isDesktop])

  useEffect(() => {
    if (!isDesktop) {
      return
    }

    if (!hasModelApiConfigured) {
      setApiSetupOpen(true)
    }
  }, [hasModelApiConfigured, isDesktop])

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
    isBusyRef.current = isBusy
  }, [isBusy])

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

  useEffect(() => {
    if (!isPetWindow) {
      return
    }

    const timer = window.setInterval(() => {
      setPetPose((current) => {
        const next = petPoses[Math.floor(Math.random() * petPoses.length)]
        if (next === current && petPoses.length > 1) {
          return petPoses[(petPoses.indexOf(current) + 1) % petPoses.length]
        }
        return next
      })
    }, 2600)

    return () => window.clearInterval(timer)
  }, [isPetWindow])

  useEffect(() => {
    if (!pendingAction) {
      return
    }

    const permissionCheck = isActionAllowedByPermissionProfile(
      settings.permissionProfile,
      pendingAction.action,
      pendingAction.riskLevel
    )
    if (!permissionCheck.allowed) {
      setPendingAction(null)
      appendAssistantMessage(
        'Pending action was cleared after permission profile changed.',
        `${settings.permissionProfile} | ${permissionCheck.reason}`
      )
    }
  }, [pendingAction, settings.permissionProfile])

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

  const executePlannedAction = async (
    action: LocalAction,
    requestText: string,
    riskLevel: string
  ) => {
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
      setLastFailedAction({
        action,
        request: requestText,
        riskLevel,
      })
      if (result.recovery_tips.length > 0) {
        appendAssistantMessage('Recovery suggestions', result.recovery_tips.join(' / '))
      }
    } else {
      setLastFailedAction(null)
    }

    return result
  }

  const runModelChat = async (requestText: string) => {
    if (!hasModelApiConfigured) {
      appendAssistantMessage(
        'Model API is not configured yet. Fill base URL, model, and API key first.',
        'Open settings and complete Model API setup'
      )
      setApiSetupOpen(true)
      return
    }

    const response = await invoke<ModelChatApiResponse>('chat_with_model_api', {
      request: {
        base_url: settings.modelBaseUrl.trim(),
        api_key: settings.modelApiKey.trim(),
        model: settings.modelName.trim(),
        user_prompt: requestText,
        system_prompt: settings.modelSystemPrompt.trim(),
        temperature: settings.modelTemperature,
        max_tokens: settings.modelMaxTokens,
      } satisfies ModelChatApiRequest,
    })

    appendAssistantMessage(
      response.content,
      `model=${response.model} | ${response.latency_ms} ms`
    )

    if (response.usage_summary) {
      appendAssistantMessage('Usage', response.usage_summary)
    }
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
      if (settings.chatMode === 'model') {
        await runModelChat(trimmed)
        return
      }

      const plan = await invoke<CommandPlan>('plan_user_request', {
        request: trimmed,
      })

      setActionQueue(plan.steps)
      appendAssistantMessage(
        plan.assistant_reply,
        `${plan.risk_level} | ${plan.can_execute_directly ? 'real action available' : 'not implemented'}`
      )

      if (plan.can_execute_directly && plan.suggested_action) {
        const permissionCheck = isActionAllowedByPermissionProfile(
          settings.permissionProfile,
          plan.suggested_action,
          plan.risk_level
        )
        if (!permissionCheck.allowed) {
          setPendingAction(null)
          setLastFailedAction(null)
          appendAssistantMessage(
            'Action blocked by permission profile.',
            `${settings.permissionProfile} | ${permissionCheck.reason}`
          )
          return
        }
      }

      if (
        isDesktop &&
        settings.autoRunSupported &&
        plan.can_execute_directly &&
        plan.suggested_action
      ) {
        if (plan.risk_level === 'high-risk') {
          setPendingAction({
            request: trimmed,
            riskLevel: plan.risk_level,
            action: plan.suggested_action,
          })
          appendAssistantMessage(
            'High-risk action detected. Click "Run high-risk action now" to execute it.',
            `${plan.suggested_action.kind} -> ${plan.suggested_action.target}`
          )
        } else {
          setPendingAction(null)
          await executePlannedAction(plan.suggested_action, trimmed, plan.risk_level)
        }
      } else if (!settings.autoRunSupported && plan.can_execute_directly) {
        if (plan.suggested_action) {
          setLastFailedAction({
            action: plan.suggested_action,
            request: trimmed,
            riskLevel: plan.risk_level,
          })
        }
        setPendingAction(null)
        appendAssistantMessage(
          'The command is supported, but auto-run is off in settings. Turn it on to execute immediately.',
          'Manual safety mode is active'
        )
      } else if (!isDesktop && plan.can_execute_directly) {
        setPendingAction(null)
        appendAssistantMessage(
          'This preview can show the plan, but only the desktop app can execute real system actions.',
          'Browser preview does not touch your desktop'
        )
      } else {
        setPendingAction(null)
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

  runRequestRef.current = runRequest

  useEffect(() => {
    if (!isDesktop || !settings.remoteBridgeEnabled || settings.chatMode !== 'command') {
      return
    }

    let cancelled = false
    const pollEverySec = Math.max(2, Math.min(30, Math.round(settings.remoteBridgePollSeconds)))

    const pollRemoteBridge = async () => {
      if (cancelled || isBusyRef.current) {
        return
      }
      try {
        const commands = await invoke<BridgeRemoteCommand[]>('bridge_pull_remote_commands', {
          limit: 1,
        })
        if (!commands || commands.length === 0) {
          return
        }

        const command = commands[0]
        appendAssistantMessage(
          `Remote command received from ${command.source}: ${command.text}`,
          `remote-id=${command.id} | ${new Date(command.received_at_ms).toLocaleString()}`
        )
        if (runRequestRef.current) {
          await runRequestRef.current(command.text)
        }
      } catch (error) {
        const detail =
          error instanceof Error ? error.message : 'Unknown remote bridge polling error'
        appendAssistantMessage('Remote bridge polling failed.', detail)
      }
    }

    void pollRemoteBridge()
    const timer = window.setInterval(() => {
      void pollRemoteBridge()
    }, pollEverySec * 1000)

    return () => {
      cancelled = true
      window.clearInterval(timer)
    }
  }, [
    isDesktop,
    settings.remoteBridgeEnabled,
    settings.remoteBridgePollSeconds,
    settings.chatMode,
  ])

  const retryLastFailedAction = async () => {
    if (!lastFailedAction || isBusy) {
      return
    }

    setIsBusy(true)
    try {
      const permissionCheck = isActionAllowedByPermissionProfile(
        settings.permissionProfile,
        lastFailedAction.action,
        lastFailedAction.riskLevel
      )
      if (!permissionCheck.allowed) {
        appendAssistantMessage(
          'Retry blocked by permission profile.',
          `${settings.permissionProfile} | ${permissionCheck.reason}`
        )
        setActionQueue([
          {
            id: makeId('retry-blocked'),
            title: `Retry blocked: ${lastFailedAction.action.label}`,
            detail: permissionCheck.reason,
            state: 'error',
          },
        ])
        return
      }

      appendAssistantMessage(
        'Retrying the last failed action.',
        `${lastFailedAction.action.kind} -> ${lastFailedAction.action.target}`
      )

      setActionQueue([
        {
          id: makeId('retry'),
          title: `Retry ${lastFailedAction.action.label}`,
          detail: `Retrying ${lastFailedAction.action.kind} (${lastFailedAction.action.target})`,
          state: 'ready',
        },
      ])

      await executePlannedAction(
        lastFailedAction.action,
        lastFailedAction.request,
        lastFailedAction.riskLevel
      )
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

  const runPendingAction = async () => {
    if (!pendingAction || isBusy) {
      return
    }

    setIsBusy(true)
    try {
      const permissionCheck = isActionAllowedByPermissionProfile(
        settings.permissionProfile,
        pendingAction.action,
        pendingAction.riskLevel
      )
      if (!permissionCheck.allowed) {
        appendAssistantMessage(
          'Pending high-risk action blocked by permission profile.',
          `${settings.permissionProfile} | ${permissionCheck.reason}`
        )
        setPendingAction(null)
        return
      }

      appendAssistantMessage(
        'Executing approved high-risk action.',
        `${pendingAction.action.kind} -> ${pendingAction.action.target}`
      )
      await executePlannedAction(
        pendingAction.action,
        pendingAction.request,
        pendingAction.riskLevel
      )
      setPendingAction(null)
    } finally {
      setIsBusy(false)
    }
  }

  const cancelPendingAction = () => {
    if (!pendingAction) {
      return
    }

    appendAssistantMessage(
      'High-risk action canceled.',
      `${pendingAction.action.kind} -> ${pendingAction.action.target}`
    )
    setPendingAction(null)
  }

  const resetConversation = () => {
    setMessages(initialMessages)
    setActionQueue(initialQueue)
    setLastFailedAction(null)
    setPendingAction(null)
  }

  const cyclePermissionProfile = () => {
    setSettings((current) => {
      const nextProfile: PermissionProfile =
        current.permissionProfile === 'safe'
          ? 'balanced'
          : current.permissionProfile === 'balanced'
            ? 'advanced'
            : 'safe'

      appendAssistantMessage(
        'Permission profile updated.',
        `${current.permissionProfile} -> ${nextProfile}`
      )

      return {
        ...current,
        permissionProfile: nextProfile,
      }
    })
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
    if (!isDesktop) return
    await invoke('minimize_to_pet')
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

  const restoreFromPet = async () => {
    if (!isDesktop) {
      return
    }
    await invoke('restore_main_from_pet')
  }

  const startPetDragging: React.MouseEventHandler<HTMLDivElement> = (event) => {
    if (!isDesktop || !windowApi || event.button !== 0) {
      return
    }
    void windowApi.startDragging()
  }

  if (isPetWindow) {
    const poseText: Partial<Record<PetPose, string>> = {
      sit: '小橘猫待命中，双击我回到聊天窗口。',
      blink: '我在眨眼观察屏幕动静。',
      stretch: '伸个懒腰，继续帮你执行任务。',
      play: '尾巴摆动中，我随时可以开工。',
      sleep: '浅睡眠巡航，唤醒后继续协助你。',
    }

    const readablePoseText: Record<PetPose, string> = {
      sit: poseText.sit ?? 'Orange cat on standby. Double-click to reopen chat.',
      blink: poseText.blink ?? 'Blink mode. Watching your screen context.',
      stretch: poseText.stretch ?? 'Stretch mode. Ready for the next command.',
      play: poseText.play ?? 'Play mode. Tail moving while waiting for tasks.',
      sleep: poseText.sleep ?? 'Light nap mode. Wake me when needed.',
      jump: 'Jump mode. High energy and ready to move.',
      dance: 'Dance mode. A trendy move loop is active.',
      listen: 'Listen mode. Focused and waiting for your instruction.',
    }

    return (
      <div
        className={`pet-widget-shell pet-widget-shell--${petPose}`}
        onMouseDown={startPetDragging}
        onDoubleClick={() => void restoreFromPet()}
      >
        <div className="pet-widget-bubble">{readablePoseText[petPose]}</div>
        <div className="pet-widget-fx" aria-hidden="true">
          <span />
          <span />
          <span />
        </div>
        <div className={`orange-cat orange-cat--${petPose}`}>
          <div className="orange-cat__tail" />
          <div className="orange-cat__body">
            <div className="orange-cat__stripe orange-cat__stripe--a" />
            <div className="orange-cat__stripe orange-cat__stripe--b" />
            <div className="orange-cat__stripe orange-cat__stripe--c" />
          </div>
          <div className="orange-cat__head">
            <div className="orange-cat__ear orange-cat__ear--left" />
            <div className="orange-cat__ear orange-cat__ear--right" />
            <div className="orange-cat__face">
              <span className="orange-cat__eye orange-cat__eye--left" />
              <span className="orange-cat__eye orange-cat__eye--right" />
              <span className="orange-cat__nose" />
            </div>
          </div>
          <div className="orange-cat__paws">
            <span />
            <span />
          </div>
        </div>
        <div className="pet-widget-actions">
          <button type="button" onClick={() => void restoreFromPet()}>
            打开聊天
          </button>
          <button
            type="button"
            onClick={() =>
              setPetPose((current) => petPoses[(petPoses.indexOf(current) + 1) % petPoses.length])
            }
          >
            换个形态
          </button>
        </div>
      </div>
    )
  }

  return (
    <div className="app-shell app-shell--gpt" onContextMenu={onContextMenu}>
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
          <div className="runtime-note">
            Chat mode: <strong>{settings.chatMode}</strong> | Model API:{' '}
            {hasModelApiConfigured ? 'configured' : 'not configured'} | Permission:{' '}
            <strong>{settings.permissionProfile}</strong>
            <br />
            {permissionText}
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
            <span className="eyebrow">xixi Desktop Assistant</span>
            <h2>xixi Chat</h2>
            <div className="header-subtitle">
              {settings.chatMode === 'model' ? 'Model chat mode' : 'Command mode'} | {permissionText}
            </div>
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
                {settings.chatMode === 'command'
                  ? 'I only execute commands that are actually wired. Try: run skill open_github'
                  : 'Model chat mode is active. Messages are sent to your configured model API.'}
              </div>
              <div className="composer__row">
                <textarea
                  value={draft}
                  onChange={(event) => setDraft(event.target.value)}
                  placeholder={
                    settings.chatMode === 'command'
                      ? 'Try: run skill open_github / open site openai.com / open folder downloads'
                      : 'Ask anything... response will come from model API'
                  }
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
              <li>Permission profile can block actions before execution</li>
            </ul>

            {lastFailedAction ? (
              <div className="failed-action-box">
                <div className="section-title">Recovery</div>
                <p>
                  Last failed action: <strong>{lastFailedAction.action.label}</strong>
                </p>
                <p>Risk level: {lastFailedAction.riskLevel}</p>
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

            {pendingAction ? (
              <div className="pending-action-box">
                <div className="section-title">High-risk gate</div>
                <p>
                  Pending: <strong>{pendingAction.action.label}</strong>
                </p>
                <p>
                  {pendingAction.action.kind}: {pendingAction.action.target}
                </p>
                <div className="pending-action-box__buttons">
                  <button
                    type="button"
                    className="retry-button"
                    onClick={() => void runPendingAction()}
                    disabled={isBusy}
                  >
                    Run high-risk action now
                  </button>
                  <button
                    type="button"
                    className="ghost-button"
                    onClick={cancelPendingAction}
                    disabled={isBusy}
                  >
                    Cancel
                  </button>
                </div>
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

            <label className="settings-toggle">
              <input
                type="checkbox"
                checked={settings.remoteBridgeEnabled}
                onChange={(event) =>
                  setSettings((current) => ({
                    ...current,
                    remoteBridgeEnabled: event.target.checked,
                  }))
                }
              />
              <span>Enable remote chat bridge polling</span>
            </label>

            <label className="settings-row">
              <span>Remote poll interval (seconds)</span>
              <input
                type="number"
                min="2"
                max="30"
                step="1"
                value={settings.remoteBridgePollSeconds}
                onChange={(event) =>
                  setSettings((current) => ({
                    ...current,
                    remoteBridgePollSeconds: Math.max(
                      2,
                      Math.min(30, Number(event.target.value) || 6)
                    ),
                  }))
                }
              />
            </label>

            <label className="settings-row">
              <span>Remote bridge folder</span>
              <input type="text" value={bridgeFolderPath} readOnly />
            </label>

            <label className="settings-row">
              <span>Permission profile</span>
              <select
                value={settings.permissionProfile}
                onChange={(event) =>
                  setSettings((current) => ({
                    ...current,
                    permissionProfile: event.target.value as PermissionProfile,
                  }))
                }
              >
                <option value="safe">Safe (web + folders only)</option>
                <option value="balanced">Balanced (apps + scripts, high-risk blocked)</option>
                <option value="advanced">Advanced (all current actions)</option>
              </select>
            </label>

            <label className="settings-row">
              <span>Chat mode</span>
              <select
                value={settings.chatMode}
                onChange={(event) =>
                  setSettings((current) => ({
                    ...current,
                    chatMode: event.target.value as 'command' | 'model',
                  }))
                }
              >
                <option value="command">Command mode (desktop actions)</option>
                <option value="model">Model chat mode (API)</option>
              </select>
            </label>

            <label className="settings-row">
              <span>Model API base URL</span>
              <input
                type="text"
                value={settings.modelBaseUrl}
                onChange={(event) =>
                  setSettings((current) => ({
                    ...current,
                    modelBaseUrl: event.target.value,
                  }))
                }
                placeholder="https://api.openai.com/v1"
              />
            </label>

            <label className="settings-row">
              <span>Model name</span>
              <input
                type="text"
                value={settings.modelName}
                onChange={(event) =>
                  setSettings((current) => ({
                    ...current,
                    modelName: event.target.value,
                  }))
                }
                placeholder="gpt-4o-mini"
              />
            </label>

            <label className="settings-row">
              <span>API key</span>
              <input
                type="password"
                value={settings.modelApiKey}
                onChange={(event) =>
                  setSettings((current) => ({
                    ...current,
                    modelApiKey: event.target.value,
                  }))
                }
                placeholder="sk-..."
              />
            </label>

            <label className="settings-row">
              <span>System prompt</span>
              <textarea
                value={settings.modelSystemPrompt}
                onChange={(event) =>
                  setSettings((current) => ({
                    ...current,
                    modelSystemPrompt: event.target.value,
                  }))
                }
                rows={4}
              />
            </label>

            <label className="settings-row">
              <span>Temperature</span>
              <input
                type="number"
                min="0"
                max="2"
                step="0.1"
                value={settings.modelTemperature}
                onChange={(event) =>
                  setSettings((current) => ({
                    ...current,
                    modelTemperature: Number(event.target.value),
                  }))
                }
              />
            </label>

            <label className="settings-row">
              <span>Max tokens</span>
              <input
                type="number"
                min="16"
                max="4096"
                step="1"
                value={settings.modelMaxTokens}
                onChange={(event) =>
                  setSettings((current) => ({
                    ...current,
                    modelMaxTokens: Number(event.target.value),
                  }))
                }
              />
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

      {apiSetupOpen ? (
        <div className="settings-overlay" role="presentation">
          <section
            className="settings-panel api-setup-panel"
            role="dialog"
            aria-label="Model API setup"
          >
            <div className="settings-panel__header">
              <div>
                <div className="section-title">Model API Setup</div>
                <h3>Fill API config for model chat</h3>
              </div>
            </div>

            <p className="settings-note">
              Command mode can run without API key. Model chat mode needs base URL, model name, and key.
            </p>

            <label className="settings-row">
              <span>Model API base URL</span>
              <input
                type="text"
                value={settings.modelBaseUrl}
                onChange={(event) =>
                  setSettings((current) => ({
                    ...current,
                    modelBaseUrl: event.target.value,
                  }))
                }
                placeholder="https://api.openai.com/v1"
              />
            </label>

            <label className="settings-row">
              <span>Model name</span>
              <input
                type="text"
                value={settings.modelName}
                onChange={(event) =>
                  setSettings((current) => ({
                    ...current,
                    modelName: event.target.value,
                  }))
                }
                placeholder="gpt-4o-mini"
              />
            </label>

            <label className="settings-row">
              <span>API key</span>
              <input
                type="password"
                value={settings.modelApiKey}
                onChange={(event) =>
                  setSettings((current) => ({
                    ...current,
                    modelApiKey: event.target.value,
                  }))
                }
                placeholder="sk-..."
              />
            </label>

            <div className="api-setup-actions">
              <button
                type="button"
                className="retry-button"
                onClick={() => {
                  if (hasModelApiConfigured) {
                    setApiSetupOpen(false)
                    setSettings((current) => ({ ...current, chatMode: 'model' }))
                    appendAssistantMessage(
                      'Model API setup saved. Switched to model chat mode.',
                      `${settings.modelName} @ ${settings.modelBaseUrl}`
                    )
                  } else {
                    appendAssistantMessage(
                      'Please fill base URL, model name, and API key before enabling model chat.'
                    )
                  }
                }}
              >
                Save and enable model chat
              </button>
              <button
                type="button"
                className="ghost-button"
                onClick={() => {
                  setApiSetupOpen(false)
                  setSettings((current) => ({ ...current, chatMode: 'command' }))
                }}
              >
                Continue with command mode
              </button>
            </div>
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
          <button type="button" onClick={cyclePermissionProfile}>
            Cycle permission profile
          </button>
          <button type="button" onClick={() => setSettingsOpen(true)}>
            Open settings
          </button>
          <button type="button" onClick={() => setApiSetupOpen(true)}>
            Open model API setup
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
