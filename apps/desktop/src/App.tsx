import { startTransition, useEffect, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
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

const initialMessages: ChatMessage[] = [
  {
    id: 'm1',
    role: 'assistant',
    author: 'xixi',
    content:
      'This build only shows real capabilities. If a command is not wired to a real desktop action yet, I will say so plainly.',
    meta: 'No fake automation. Real actions only.',
  },
  {
    id: 'm2',
    role: 'assistant',
    author: 'xixi',
    content:
      'Supported now: open QMDownload, open xixi folder, open GitHub, open weather, open Chrome, open Edge, open Notepad, open Explorer.',
    meta: 'These commands run through the desktop shell.',
  },
]

const initialQueue: ActionItem[] = [
  {
    id: 'boot-1',
    title: 'Desktop shell',
    detail: 'Tauri shell is connected to the chat workspace',
    state: 'done',
  },
  {
    id: 'boot-2',
    title: 'Command planner',
    detail: 'Only supported desktop actions are mapped to execution',
    state: 'done',
  },
  {
    id: 'boot-3',
    title: 'Action runner',
    detail: 'Safe local actions can execute immediately',
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

function App() {
  const isDesktop = '__TAURI_INTERNALS__' in window
  const runtimeMode = isDesktop ? 'Desktop shell' : 'Browser preview'
  const [desktopProfile, setDesktopProfile] = useState<DesktopProfile | null>(null)
  const [messages, setMessages] = useState(initialMessages)
  const [actionQueue, setActionQueue] = useState(initialQueue)
  const [draft, setDraft] = useState('Open GitHub')
  const [isBusy, setIsBusy] = useState(false)

  useEffect(() => {
    if (!isDesktop) {
      return
    }

    invoke<DesktopProfile>('get_desktop_profile')
      .then((profile) => setDesktopProfile(profile))
      .catch(() => setDesktopProfile(null))
  }, [isDesktop])

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
        `${plan.risk_level} | ${plan.can_execute_directly ? 'direct run' : 'not executable yet'}`
      )

      if (isDesktop && plan.can_execute_directly && plan.suggested_action) {
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
      } else if (!isDesktop && plan.can_execute_directly) {
        appendAssistantMessage(
          'The request is supported, but this browser preview cannot touch your desktop. Launch the Tauri app to run it for real.',
          'Preview mode does not execute system actions'
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

  return (
    <div className="app-shell">
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
          <h1>Real Desktop Actions</h1>
          <p className="pet-card__summary">
            This screen now reflects only what xixi can truly do in the current build.
          </p>
          <div className="pet-card__status">
            <span className="status-dot" />
            {isBusy ? 'Busy: planning and running a real action' : 'Idle: waiting for a supported command'}
          </div>
          <div className="self-talk">
            {isBusy
              ? '"I am checking whether this request maps to a real desktop action."'
              : '"If a command is not wired yet, I will say not implemented instead of faking it."'}
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

      <main className="workspace">
        <header className="workspace-header">
          <div>
            <span className="eyebrow">Phase 1 Workspace</span>
            <h2>Chat to desktop action</h2>
          </div>
          <div className="window-actions" aria-label="Window controls">
            <span />
            <span />
            <span />
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
                Try a real command from the supported list. Unsupported requests will stay honest and explicit.
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
              <li>No pretend software control</li>
              <li>Unsupported actions are reported clearly</li>
              <li>Every enabled action goes through a real desktop command</li>
            </ul>
          </aside>
        </section>
      </main>
    </div>
  )
}

export default App
