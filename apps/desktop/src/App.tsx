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
      '欢迎回来。我已经准备好用自然语言帮你操作电脑、整理任务，并把每一步都解释成你能看懂的话。',
    meta: '小事直接执行，大事先确认',
  },
  {
    id: 'm2',
    role: 'assistant',
    author: '机灵猫人格',
    content:
      '这一版已经接上了桌面动作链。你可以先试试“打开 D 盘下载区”或者“帮我打开 GitHub”。',
    meta: '支持的安全动作会自动执行',
  },
]

const initialQueue: ActionItem[] = [
  {
    id: 'a1',
    title: '聊天工作台',
    detail: '主界面和桌面壳已经连通',
    state: 'done',
  },
  {
    id: 'a2',
    title: '自然语言理解',
    detail: '准备把你的口语指令拆成动作计划',
    state: 'ready',
  },
  {
    id: 'a3',
    title: '本地执行器',
    detail: '将安全的小动作转成桌面操作',
    state: 'waiting',
  },
]

const personas = [
  {
    name: '陪伴猫',
    description: '温柔、轻提醒、适合日常陪伴',
  },
  {
    name: '机灵猫',
    description: '擅长执行和拆任务，语气更利落',
  },
  {
    name: '研究猫',
    description: '偏爱资料检索、GitHub 学习与总结',
  },
]

const quickActions = [
  '打开 D 盘下载区',
  '打开 xixi 项目目录',
  '帮我打开 GitHub',
  '查看今天的天气',
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
  const [draft, setDraft] = useState('帮我打开 GitHub')
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
          author: '你',
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
        `${plan.risk_level} · ${plan.can_execute_directly ? '可直接执行' : '需要确认'}`
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
          result.details.length > 0 ? result.details.join(' / ') : '已完成桌面动作'
        )
      } else if (!isDesktop && plan.can_execute_directly) {
        appendAssistantMessage(
          '当前是浏览器预览模式，我已经把动作计划好了。等以桌面应用启动后，我会直接替你执行。',
          '预览模式不会真的操作电脑'
        )
      }
    } catch (error) {
      const detail = error instanceof Error ? error.message : '未知错误'
      appendAssistantMessage(
        '这次动作没有成功执行，我已经把错误状态保留下来了，方便下一轮继续修。',
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
            <span className="pet-card__badge">xixi / 晰晰</span>
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
          <h1>小橘猫桌面智能体</h1>
          <p className="pet-card__summary">
            双击桌面宠物进入聊天主界面。这里已经不是静态样子货，而是第一版可理解指令、会组织动作、能触发桌面执行的工作台。
          </p>
          <div className="pet-card__status">
            <span className="status-dot" />
            {isBusy ? '忙碌中: 正在理解你的新指令' : '待命中: 准备执行新的桌面动作'}
          </div>
          <div className="self-talk">
            {isBusy
              ? '“我先把你的话拆成步骤，再决定是不是能直接帮你做。”'
              : '“现在你可以直接对我说人话了，我会先理解再动手。”'}
          </div>
          {desktopProfile ? (
            <div className="runtime-note">
              <strong>桌面连接已打通</strong>
              <ul>
                {desktopProfile.notes.map((note) => (
                  <li key={note}>{note}</li>
                ))}
              </ul>
            </div>
          ) : null}
        </div>

        <section className="sidebar-section">
          <div className="section-title">人格切换</div>
          <div className="persona-list">
            {personas.map((persona, index) => (
              <article
                key={persona.name}
                className={index === 1 ? 'persona-card is-active' : 'persona-card'}
              >
                <div>
                  <strong>{persona.name}</strong>
                  <p>{persona.description}</p>
                </div>
                <button type="button">切换</button>
              </article>
            ))}
          </div>
        </section>
      </aside>

      <main className="workspace">
        <header className="workspace-header">
          <div>
            <span className="eyebrow">Phase 1 Workspace</span>
            <h2>聊天理解 + 电脑执行</h2>
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
                试试这些说法: “打开 D 盘下载区” “帮我打开 GitHub” “查看今天的天气”
              </div>
              <div className="composer__row">
                <textarea
                  value={draft}
                  onChange={(event) => setDraft(event.target.value)}
                  placeholder="直接用自然语言告诉 xixi 你想做什么"
                />
                <button
                  type="button"
                  onClick={() => void runRequest(draft)}
                  disabled={isBusy}
                >
                  {isBusy ? '处理中' : '发送'}
                </button>
              </div>
            </footer>
          </div>

          <aside className="action-panel">
            <div className="section-title">执行队列</div>
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

            <div className="section-title">第一版能力</div>
            <ul className="goal-list">
              <li>像 GPT 一样稳定的聊天主窗口</li>
              <li>自然语言转成安全的小动作计划</li>
              <li>桌面宠物状态和自言自语联动</li>
              <li>后续接入技能和智能体注册入口</li>
            </ul>
          </aside>
        </section>
      </main>
    </div>
  )
}

export default App
