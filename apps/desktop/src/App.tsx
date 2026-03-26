import { useEffect, useState } from 'react'
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
  state: 'ready' | 'running' | 'waiting'
}

type DesktopProfile = {
  app_name: string
  runtime: string
  action_mode: string
  notes: string[]
}

const messages: ChatMessage[] = [
  {
    id: 'm1',
    role: 'assistant',
    author: 'xixi',
    content:
      '欢迎回来。我已经准备好用自然语言帮你操作电脑、整理任务和解释每一步在做什么。',
    meta: '小事直接执行，大事先确认',
  },
  {
    id: 'm2',
    role: 'user',
    author: '你',
    content: '帮我打开音乐播放器，找一首《真的爱你》。',
  },
  {
    id: 'm3',
    role: 'assistant',
    author: '机灵猫人格',
    content:
      '收到，我先检查可用播放器，再尝试搜索歌曲。执行时我会在桌面宠物状态里显示忙碌和自言自语。',
    meta: '已拆分为 打开应用 / 搜索歌曲 / 开始播放',
  },
]

const actionQueue: ActionItem[] = [
  {
    id: 'a1',
    title: '打开应用',
    detail: '解析默认播放器和可执行路径',
    state: 'running',
  },
  {
    id: 'a2',
    title: '自然语言理解',
    detail: '将“真的爱你”转成播放器搜索动作',
    state: 'ready',
  },
  {
    id: 'a3',
    title: '安全确认',
    detail: '当前任务属于低风险，允许直接执行',
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
  '打开 Chrome',
  '整理 D 盘文件',
  '查看今天的天气',
  '总结 GitHub 学习收获',
]

function App() {
  const runtimeMode =
    '__TAURI_INTERNALS__' in window ? 'Desktop shell' : 'Browser preview'
  const [desktopProfile, setDesktopProfile] = useState<DesktopProfile | null>(null)

  useEffect(() => {
    if (!('__TAURI_INTERNALS__' in window)) {
      return
    }

    invoke<DesktopProfile>('get_desktop_profile')
      .then((profile) => setDesktopProfile(profile))
      .catch(() => setDesktopProfile(null))
  }, [])

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
            双击桌面宠物进入聊天主界面。这里是第一版控制台，用来承载对话、执行状态和后续的技能系统。
          </p>
          <div className="pet-card__status">
            <span className="status-dot" />
            忙碌中: 正在准备桌面自动化入口
          </div>
          <div className="self-talk">
            “我先把聊天窗口和执行队列搭好，后面就能真正开始替你做事了。”
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
            <button key={action} type="button">
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
                以后你可以直接说: “帮我打开某某软件” “整理 D 盘文件” “查天气提醒我”
              </div>
              <div className="composer__row">
                <textarea
                  readOnly
                  value="第一版先把桌面底座搭好，下一步接真实的本地执行器和模型路由。"
                />
                <button type="button">发送</button>
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

            <div className="section-title">第一版目标</div>
            <ul className="goal-list">
              <li>像 GPT 一样稳定的聊天主窗口</li>
              <li>自然语言转成本地电脑动作</li>
              <li>桌面宠物状态和自言自语联动</li>
              <li>技能与智能体注册入口</li>
            </ul>
          </aside>
        </section>
      </main>
    </div>
  )
}

export default App
