const CATEGORY_KEYS = ['web_server', 'database', 'development', 'system', 'other']

const state = {
  snapshot: null,
  selectedDirectory: null,
  selectedSessionId: null,
  selectedAgentId: null,
  expandedProjects: new Set(),
  error: null,
  activeTab: 'dashboard',
  portData: null,
  portSearch: '',
  portCategories: new Set(), // empty = show all (same as 전체 checked)
}

const polling = {
  snapshotTimer: null,
  portTimer: null,
  runningDelay: 5000,
  idleDelay: 10000,
  hiddenDelay: 15000,
  portActiveDelay: 10000,
  isVisible: !document.hidden,
}

let isSnapshotLoading = false
let isPortLoading = false

const invoke = window.__TAURI__?.core?.invoke.bind(window.__TAURI__.core) ?? (() => Promise.reject('No Tauri'))

function formatAge(ageSec) {
  if (ageSec == null) return 'n/a'
  if (ageSec < 60) return `${ageSec}s ago`
  const min = Math.floor(ageSec / 60)
  const sec = ageSec % 60
  return `${min}m ${sec}s ago`
}

function relativeTimeKorean(dateStr) {
  const diffSec = Math.floor((Date.now() - new Date(dateStr).getTime()) / 1000)
  if (diffSec < 5) return '방금 전'
  if (diffSec < 60) return `${diffSec}초 전`
  const min = Math.floor(diffSec / 60)
  if (min < 60) return `${min}분 전`
  const hr = Math.floor(min / 60)
  if (hr < 24) return `${hr}시간 전`
  return `${Math.floor(hr / 24)}일 전`
}

function formatDuration(sec) {
  if (sec == null) return 'n/a'
  if (sec < 60) return `${sec}s`
  const hours = Math.floor(sec / 3600)
  const minutes = Math.floor((sec % 3600) / 60)
  if (hours) return `${hours}h ${minutes}m`
  return `${minutes}m ${sec % 60}s`
}

function statusBadge(status, label) {
  return `<span class="badge badge--${status}">${label || status}</span>`
}

function escapeHtml(value) {
  return String(value ?? '')
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
}

function statusColor(status) {
  return { running: '#4fd1c5', delayed: '#f6ad55', stalled: '#fc8181', failed: '#fb7185', completed: '#68d391' }[status] || '#90a0bd'
}

function statusLabel(s) {
  if (s === 'running') return 'Running'
  if (s === 'delayed') return 'Delayed'
  if (s === 'stalled') return 'Stalled'
  if (s === 'failed') return 'Failed'
  if (s === 'completed') return 'Completed'
  return s
}

function sessionOverallStatus(session) {
  if (session.statusCounts.stalled > 0) return 'stalled'
  if (session.statusCounts.running > 0) return 'running'
  if (session.statusCounts.failed > 0) return 'failed'
  return 'completed'
}

function ensureSelection(snapshot) {
  const sessions = snapshot.sessions || []
  if (!sessions.length) {
    state.selectedDirectory = null
    state.selectedSessionId = null
    state.selectedAgentId = null
    return
  }
  if (!state.selectedDirectory || !sessions.some(s => s.directory === state.selectedDirectory)) {
    state.selectedDirectory = sessions[0].directory
  }
  const dirSessions = sessions.filter(s => s.directory === state.selectedDirectory)
  const currentSession = dirSessions.find((s) => s.id === state.selectedSessionId) || dirSessions[0]
  state.selectedSessionId = currentSession ? currentSession.id : null
  if (!currentSession) { state.selectedAgentId = null; return }
  const allAgents = [...currentSession.agents, ...(currentSession.children || []).flatMap(c => c.agents)]
  const currentAgent = allAgents.find((a) => a.id === state.selectedAgentId) || allAgents[0] || null
  state.selectedAgentId = currentAgent?.id || null
}

function findAgentGlobally(agentId) {
  if (!state.snapshot) return null
  for (const session of state.snapshot.sessions) {
    const found = session.agents.find(a => a.id === agentId)
    if (found) return found
    for (const child of (session.children || [])) {
      const found2 = child.agents.find(a => a.id === agentId)
      if (found2) return found2
    }
  }
  return null
}

function groupByDirectory(sessions) {
  const groups = new Map()
  for (const session of sessions) {
    const dir = session.directory || 'unknown'
    const basename = dir.split('/').pop() || dir
    if (!groups.has(dir)) groups.set(dir, { name: basename, directory: dir, sessions: [] })
    groups.get(dir).sessions.push(session)
  }
  return Array.from(groups.values()).sort((a, b) => {
    const aLatest = Math.max(...a.sessions.map(s => new Date(s.lastActivityAt || 0).getTime()))
    const bLatest = Math.max(...b.sessions.map(s => new Date(s.lastActivityAt || 0).getTime()))
    return bLatest - aLatest
  })
}

function renderTopBar(snapshot) {
  document.getElementById('running-agents').textContent = snapshot.summary.runningAgents
  document.getElementById('stalled-agents').textContent = snapshot.summary.suspectedStalled
  document.getElementById('last-refreshed').textContent = relativeTimeKorean(snapshot.generatedAt)
}

function renderProjects(snapshot) {
  const container = document.getElementById('projects-list')
  const projects = groupByDirectory(snapshot.sessions)

  document.getElementById('project-count').textContent = `${projects.length} projects`

  container.innerHTML = projects.map(project => {
    const isExpanded = state.expandedProjects.has(project.directory)
    const runningCount = project.sessions.reduce((sum, s) => sum + s.statusCounts.running, 0)
    const stalledCount = project.sessions.reduce((sum, s) => sum + s.statusCounts.stalled, 0)
    return `
      <div class="project-card ${project.directory === state.selectedDirectory ? 'project-card--active' : ''}" data-directory="${escapeHtml(project.directory)}" data-collapsed="${!isExpanded}">
        <div class="project-card__header" data-dir="${escapeHtml(project.directory)}">
          <div class="project-card__icon">${escapeHtml(project.name.charAt(0).toUpperCase())}</div>
          <div class="project-card__info">
            <div class="project-card__name">${escapeHtml(project.name)}</div>
            <div class="project-card__dir">${escapeHtml(project.directory)}</div>
            <div class="project-card__meta">
              <span>${project.sessions.length} session${project.sessions.length !== 1 ? 's' : ''}</span>
              ${runningCount ? `<span class="project-card__stat project-card__stat--running">${runningCount} running</span>` : ''}
              ${stalledCount ? `<span class="project-card__stat project-card__stat--stalled">${stalledCount} stalled</span>` : ''}
            </div>
          </div>
          <div class="project-card__chevron">&#9654;</div>
        </div>
        <div class="project-sessions">
          ${project.sessions.map(session => {
            const overall = sessionOverallStatus(session)
            return `
              <div class="project-session-item ${session.id === state.selectedSessionId ? 'is-active' : ''}" data-session-id="${session.id}">
                <span class="status-dot" style="background:${statusColor(overall)}"></span>
                <span class="project-session-item__name">${escapeHtml(session.name)}</span>
                <span class="project-session-item__agents">${session.agents.length}</span>
              </div>
            `
          }).join('')}
        </div>
      </div>
    `
  }).join('')

  container.querySelectorAll('.project-card__header').forEach(el => {
    el.addEventListener('click', () => {
      const dir = el.dataset.dir
      state.selectedDirectory = dir
      state.selectedSessionId = null
      state.selectedAgentId = null
      if (state.expandedProjects.has(dir)) {
        state.expandedProjects.delete(dir)
      } else {
        state.expandedProjects.add(dir)
      }
      render()
    })
  })

  container.querySelectorAll('.project-session-item').forEach(el => {
    el.addEventListener('click', (e) => {
      e.stopPropagation()
      const sessionId = el.dataset.sessionId
      const session = snapshot.sessions.find(s => s.id === sessionId)
      if (session) state.selectedDirectory = session.directory
      state.selectedSessionId = sessionId
      state.selectedAgentId = null
      render()
    })
  })
}

function agentCardHtml(agent, isActive, isSubAgent) {
  const tools = (agent.tools || []).slice(0, 4).map(t => `<span class="tool-badge">${escapeHtml(t)}</span>`).join('')
  return `
    <article class="agent-card ${isActive ? 'is-active' : ''} ${isSubAgent ? 'agent-card--sub' : ''}" data-status="${agent.status}" data-agent-id="${agent.id}">
      <div class="agent-card__header">
        <div>
          <div class="agent-card__name">${escapeHtml(agent.name)}</div>
          <div class="agent-card__model">${escapeHtml(agent.model)}</div>
        </div>
        ${statusBadge(agent.status, statusLabel(agent.status))}
      </div>
      <div class="agent-card__task">${escapeHtml(agent.task)}</div>
      <div class="agent-card__meta">
        <span>${formatAge(agent.lastEventAgeSec)}</span>
        <span>${formatDuration(agent.durationSec)}</span>
      </div>
      ${tools ? `<div class="tools">${tools}</div>` : ''}
    </article>
  `
}

function renderAllSessions(snapshot) {
  const container = document.getElementById('sessions-main')
  const filtered = snapshot.sessions.filter(s => s.directory === state.selectedDirectory)

  container.innerHTML = filtered.map(session => {
    const overall = sessionOverallStatus(session)
    const dirBasename = (session.directory || '').split('/').pop() || ''
    const allAgents = [...session.agents, ...(session.children || []).flatMap(c => c.agents)]
    const dots = allAgents.slice(0, 8).map(a => `<span class="status-dot" style="background:${statusColor(a.status)}"></span>`).join('')

    const sortedAgents = [...session.agents].sort((a, b) => (a.lastEventAgeSec ?? Infinity) - (b.lastEventAgeSec ?? Infinity))
    const agentsHtml = sortedAgents.map(a => agentCardHtml(a, a.id === state.selectedAgentId)).join('')

    let childrenAgentsHtml = ''
    if (session.children && session.children.length > 0) {
      const sortedChildren = session.children.flatMap(child =>
        [...child.agents].sort((a, b) => (a.lastEventAgeSec ?? Infinity) - (b.lastEventAgeSec ?? Infinity))
      )
      childrenAgentsHtml = sortedChildren.map(a => agentCardHtml(a, a.id === state.selectedAgentId, true)).join('')
    }

    const childCount = (session.children || []).length

    return `
      <div class="session-group ${session.id === state.selectedSessionId ? 'session-group--active' : ''}" id="session-group-${session.id}" data-session-id="${session.id}">
        <div class="session-group__header">
          <div class="session-group__info">
            <div class="session-group__name">${escapeHtml(session.name)}</div>
            <div class="session-group__meta">
              <span class="session-group__dir">${escapeHtml(dirBasename)}</span>
              <span>${allAgents.length} agents</span>
              ${childCount ? `<span>${childCount} sub-sessions</span>` : ''}
              <span>${formatDuration(session.durationSec)}</span>
            </div>
          </div>
          <div class="session-group__status">
            ${statusBadge(overall, statusLabel(overall))}
            <div class="status-dots">${dots}</div>
          </div>
        </div>
        <div class="session-group__agents">
          ${agentsHtml}
          ${childrenAgentsHtml}
        </div>
      </div>
    `
  }).join('')

  container.querySelectorAll('[data-agent-id]').forEach(el => {
    el.addEventListener('click', () => {
      state.selectedAgentId = el.dataset.agentId
      const sessionGroup = el.closest('.session-group')
      if (sessionGroup) {
        state.selectedSessionId = sessionGroup.dataset.sessionId
        const session = snapshot.sessions.find(s => s.id === state.selectedSessionId)
        if (session) state.selectedDirectory = session.directory
      }
      render()
    })
  })
}

function renderDetail() {
  const container = document.getElementById('detail-panel')
  const agent = findAgentGlobally(state.selectedAgentId)
  if (!agent) {
    container.innerHTML = '<div class="panel--detail-scroll"><div class="empty-state">에이전트를 선택하면 상세가 표시됩니다.</div></div>'
    return
  }

  const logCount = (agent.recentLogs || []).length
  const logsHtml = (agent.recentLogs || []).map((log) => `
    <article class="log-item">
      <div class="log-item__top">
        <span>${new Date(log.time).toLocaleTimeString()}</span>
        <span>${escapeHtml(log.level.toUpperCase())}</span>
      </div>
      <div class="log-item__message">${escapeHtml(log.message)}</div>
    </article>
  `).join('') || '<div class="empty-state">최근 로그가 없습니다.</div>'

  container.innerHTML = `
    <div class="panel--detail-scroll">
      <section class="detail-card">
        <div class="detail-card__header">
          <div>
            <div class="agent-card__name">${escapeHtml(agent.name)}</div>
            <div class="detail-card__model">${escapeHtml(agent.model)}</div>
          </div>
          ${statusBadge(agent.status, statusLabel(agent.status))}
        </div>
        <div class="detail-card__task">${escapeHtml(agent.task)}</div>
        <div class="detail-grid">
          <div class="detail-stat"><div class="detail-stat__label">Last activity</div><div class="detail-stat__value">${formatAge(agent.lastEventAgeSec)}</div></div>
          <div class="detail-stat"><div class="detail-stat__label">Duration</div><div class="detail-stat__value">${formatDuration(agent.durationSec)}</div></div>
          <div class="detail-stat"><div class="detail-stat__label">Started</div><div class="detail-stat__value">${agent.startedAt ? new Date(agent.startedAt).toLocaleTimeString() : 'n/a'}</div></div>
          <div class="detail-stat"><div class="detail-stat__label">Tools</div><div class="detail-stat__value">${escapeHtml((agent.tools || []).join(', ') || 'n/a')}</div></div>
        </div>
      </section>
      <div class="panel--detail-body">
        <div class="logs-header">
          <span>Logs (${logCount})</span>
          ${logCount > 5 ? `<button class="logs-toggle" data-expanded="false">Show all ${logCount}</button>` : ''}
        </div>
        <div class="logs" data-collapsed="${logCount > 5}">
          ${logsHtml}
        </div>
      </div>
    </div>
  `

  const toggleBtn = container.querySelector('.logs-toggle')
  if (toggleBtn) {
    toggleBtn.addEventListener('click', () => {
      const expanded = toggleBtn.dataset.expanded === 'true'
      toggleBtn.dataset.expanded = String(!expanded)
      toggleBtn.textContent = expanded ? `Show all ${logCount}` : 'Show less'
      const logsEl = container.querySelector('.logs')
      logsEl.dataset.collapsed = String(expanded)
    })
  }
}

function renderError(message) {
  document.getElementById('sessions-main').innerHTML = `<div class="error-state">${escapeHtml(message)}</div>`
  document.getElementById('detail-panel').innerHTML = `<div class="panel--detail-scroll"><div class="error-state">${escapeHtml(message)}</div></div>`
}

function renderHeavy() {
  if (state.error) { renderError(state.error); return }
  if (!state.snapshot) return

  const sp = {}
  const pl = document.getElementById('projects-list')
  const sm = document.getElementById('sessions-main')
  const ds = document.querySelector('.panel--detail-scroll')
  if (pl) sp.projects = pl.scrollTop
  if (sm) sp.sessions = sm.scrollTop
  if (ds) sp.detail = ds.scrollTop

  renderProjects(state.snapshot)
  renderAllSessions(state.snapshot)
  renderDetail()

  requestAnimationFrame(() => {
    const pl2 = document.getElementById('projects-list')
    const sm2 = document.getElementById('sessions-main')
    const ds2 = document.querySelector('.panel--detail-scroll')
    if (pl2 && sp.projects != null) pl2.scrollTop = sp.projects
    if (sm2 && sp.sessions != null) sm2.scrollTop = sp.sessions
    if (ds2 && sp.detail != null) ds2.scrollTop = sp.detail
  })
}

function render() {
  if (!state.snapshot) return
  renderTopBar(state.snapshot)
  renderHeavy()
}

// DB status indicator
const dbStatus = document.getElementById('db-status');

// ── SleepGuard UI ──
let _sleepCountdownTimer = null
let _sleepNextPollAt = 0

function updateSleepUI(status) {
  const icon = document.getElementById('sleep-icon')
  const label = document.getElementById('sleep-label')
  if (!icon || !label) return
  if (status.isPreventing) {
    icon.textContent = '🟢'
    label.textContent = '절전 방지'
  } else {
    icon.textContent = '⚪'
    label.textContent = '시스템 설정에 따름'
  }
  updateSleepDebug(status)
}

function formatDurationKorean(sec) {
  if (sec == null || sec < 0) return '—'
  const h = Math.floor(sec / 3600)
  const m = Math.floor((sec % 3600) / 60)
  const s = Math.floor(sec % 60)
  if (h > 0) return `${h}시간 ${m}분`
  if (m > 0) return `${m}분 ${s}초`
  return `${s}초`
}

function formatTimeShort(isoStr) {
  if (!isoStr) return '—'
  try {
    return new Date(isoStr).toLocaleTimeString('ko-KR', { hour: '2-digit', minute: '2-digit', second: '2-digit' })
  } catch (_) {
    return isoStr
  }
}

function updateSleepDebug(status) {
  const el = (id) => document.getElementById(id)
  const preventingEl = el('dbg-preventing')
  const modeEl = el('dbg-mode')
  const agentsEl = el('dbg-agents')
  const displayEl = el('dbg-display')
  const idleEl = el('dbg-idle')
  const timeLabelEl = el('dbg-time-label')
  const timeEl = el('dbg-time')
  const durationLabelEl = el('dbg-duration-label')
  const durationEl = el('dbg-duration')
  const countdownEl = el('dbg-countdown')
  const expectEl = el('dbg-expect')
  const toggleBtn = el('sleep-manual-toggle')

  if (!preventingEl) return

  const active = status.isPreventing
  const agents = status.activeAgents ?? 0

  // 상태
  preventingEl.textContent = active ? '🟢 절전 방지 중' : '⚪ 시스템 설정에 따름'
  preventingEl.style.color = active ? '#4fd1c5' : '#6f7f9b'

  // 모드
  modeEl.textContent = active ? '시스템 설정 무시 · 절전 차단' : 'OS 기본 절전 정책 따름'
  modeEl.style.color = active ? '#90a0bd' : '#6f7f9b'

  // 활성 에이전트
  agentsEl.textContent = agents > 0 ? `${agents}개` : '0개'
  agentsEl.style.color = agents > 0 ? '#4fd1c5' : '#6f7f9b'

  // display / idle
  displayEl.textContent = status.display ? 'ON' : 'OFF'
  displayEl.style.color = status.display ? '#4fd1c5' : '#6f7f9b'
  idleEl.textContent = status.idle ? 'ON' : 'OFF'
  idleEl.style.color = status.idle ? '#4fd1c5' : '#6f7f9b'

  // 시각 / 유지시간
  if (active && status.startedAt) {
    timeLabelEl.textContent = '시작 시각'
    timeEl.textContent = formatTimeShort(status.startedAt)
    const durSec = Math.floor((Date.now() - new Date(status.startedAt).getTime()) / 1000)
    durationLabelEl.textContent = '유지 시간'
    durationEl.textContent = formatDurationKorean(durSec)
  } else if (!active && status.lastChangedAt) {
    timeLabelEl.textContent = '해제 시각'
    timeEl.textContent = formatTimeShort(status.lastChangedAt)
    const durSec = Math.floor((Date.now() - new Date(status.lastChangedAt).getTime()) / 1000)
    durationLabelEl.textContent = '경과 시간'
    durationEl.textContent = formatDurationKorean(durSec)
  } else {
    timeLabelEl.textContent = '시각'
    timeEl.textContent = '—'
    durationLabelEl.textContent = '시간'
    durationEl.textContent = '—'
  }

  // 예상
  expectEl.textContent = active ? '수동 끄기 → 시스템 설정에 따름' : '수동 켜기 → 절전 방지'
  expectEl.style.color = '#6f7f9b'

  // 수동 토글 버튼
  if (toggleBtn) {
    toggleBtn.textContent = active ? '수동 끄기' : '수동 켜기'
    toggleBtn.className = active ? 'sleep-debug__toggle sleep-debug__toggle--off' : 'sleep-debug__toggle sleep-debug__toggle--on'
  }
}

function startSleepCountdown() {
  if (_sleepCountdownTimer) clearInterval(_sleepCountdownTimer)
  const countdownEl = document.getElementById('dbg-countdown')
  if (!countdownEl) return

  _sleepCountdownTimer = setInterval(() => {
    const remaining = Math.max(0, Math.ceil((_sleepNextPollAt - Date.now()) / 1000))
    const panel = document.getElementById('sleep-debug')
    if (!panel || panel.classList.contains('hidden')) return
    countdownEl.textContent = remaining > 0 ? `${remaining}초 후 상태 확인` : '확인 중...'
    countdownEl.style.color = remaining <= 3 ? '#f6ad55' : '#90a0bd'
  }, 500)
}

function initSleepDebug() {
  const indicator = document.getElementById('sleep-indicator')
  const debugPanel = document.getElementById('sleep-debug')
  const closeBtn = document.getElementById('sleep-debug-close')
  const toggleBtn = document.getElementById('sleep-manual-toggle')
  if (!indicator || !debugPanel) return

  indicator.style.cursor = 'pointer'
  indicator.addEventListener('click', (e) => {
    e.stopPropagation()
    debugPanel.classList.toggle('hidden')
  })
  if (closeBtn) {
    closeBtn.addEventListener('click', (e) => {
      e.stopPropagation()
      debugPanel.classList.add('hidden')
    })
  }
  // Manual toggle
  if (toggleBtn) {
    toggleBtn.addEventListener('click', async (e) => {
      e.stopPropagation()
      const current = await invoke('get_sleep_status').catch(() => null)
      if (!current) return
      const newPrevent = !current.isPreventing
      try {
        const result = await invoke('set_sleep_prevention', {
          prevent: newPrevent,
          reason: newPrevent ? 'manual' : 'manual',
          activeAgents: newPrevent ? -1 : 0,
        })
        updateSleepUI(result)
      } catch (_) {}
    })
  }
  // Close on outside click
  document.addEventListener('click', (e) => {
    if (!debugPanel.contains(e.target) && !indicator.contains(e.target)) {
      debugPanel.classList.add('hidden')
    }
  })
  startSleepCountdown()
}

function getSnapshotDelay() {
  if (!polling.isVisible) return polling.hiddenDelay
  const running = state.snapshot?.summary?.runningAgents ?? 0
  return running > 0 ? polling.runningDelay : polling.idleDelay
}

function scheduleSnapshotPolling() {
  clearTimeout(polling.snapshotTimer)
  const delay = getSnapshotDelay()
  _sleepNextPollAt = Date.now() + delay
  polling.snapshotTimer = setTimeout(loadSnapshot, delay)
}

function schedulePortPolling() {
  clearTimeout(polling.portTimer)
  if (!polling.isVisible || state.activeTab !== 'portkiller') return
  polling.portTimer = setTimeout(async () => {
    await loadPorts()
    schedulePortPolling()
  }, polling.portActiveDelay)
}

document.addEventListener('visibilitychange', () => {
  polling.isVisible = !document.hidden
  if (polling.isVisible) {
    loadSnapshot()
    if (state.activeTab === 'portkiller') loadPorts()
  } else {
    scheduleSnapshotPolling()
    schedulePortPolling()
  }
})

// ── Tab Navigation ──
function initTabs() {
  document.querySelectorAll('.tab-btn').forEach(btn => {
    btn.addEventListener('click', () => {
      const tab = btn.dataset.tab
      state.activeTab = tab
      document.querySelectorAll('.tab-btn').forEach(b => b.classList.toggle('active', b.dataset.tab === tab))
      document.querySelectorAll('.tab-content').forEach(c => c.classList.toggle('hidden', c.id !== `tab-${tab}`))
      if (tab === 'portkiller') {
        loadPorts()
        schedulePortPolling()
      } else {
        schedulePortPolling()
      }
    })
  })
  const searchInput = document.getElementById('port-search')
  let portSearchTimer = null
  if (searchInput) {
    searchInput.addEventListener('input', () => {
      state.portSearch = searchInput.value
      clearTimeout(portSearchTimer)
      portSearchTimer = setTimeout(renderPorts, 200)
    })
  }
  const refreshBtn = document.getElementById('port-refresh')
  if (refreshBtn) {
    refreshBtn.addEventListener('click', loadPorts)
  }

  // Category filter checkboxes
  const catAll = document.getElementById('cat-all')
  const catSpecific = document.querySelectorAll('.port-filter__item input[data-category]:not(#cat-all)')

  if (catAll) {
    catAll.addEventListener('change', () => {
      if (catAll.checked) {
        // Reset to show all
        state.portCategories.clear()
        catSpecific.forEach(cb => { cb.checked = false })
      } else {
        // Unchecking 전체 alone: check all specific as fallback
        catAll.checked = true
      }
      renderPorts()
    })
  }

  catSpecific.forEach(cb => {
    cb.addEventListener('change', () => {
      const cat = cb.dataset.category
      if (cb.checked) {
        state.portCategories.add(cat)
        // Uncheck 전체 when specific categories are selected
        if (catAll) catAll.checked = false
      } else {
        state.portCategories.delete(cat)
        // If no specific categories checked, revert to 전체
        if (state.portCategories.size === 0 && catAll) {
          catAll.checked = true
        }
      }
      renderPorts()
    })
  })
}

// ── Port Killer ──
async function loadPorts() {
  if (isPortLoading) return
  isPortLoading = true
  try {
    const result = await invoke('scan_ports')
    state.portData = result
    renderPorts()
  } catch(e) {
    const tbody = document.getElementById('port-table-body')
    if (tbody) tbody.innerHTML = `<tr><td colspan="5" class="port-error">포트 스캔 실패: ${escapeHtml(String(e))}</td></tr>`
  } finally {
    isPortLoading = false
  }
}

function renderPorts() {
  const tbody = document.getElementById('port-table-body')
  if (!tbody || !state.portData) return

  let ports = state.portData.ports || []

  // Category filter: if state.portCategories is non-empty, only those categories
  if (state.portCategories.size > 0) {
    ports = ports.filter(p => state.portCategories.has(p.category))
  }

  // Search filter
  const search = state.portSearch.toLowerCase()
  if (search) {
    ports = ports.filter(p =>
      String(p.port).includes(search) ||
      (p.processName || '').toLowerCase().includes(search) ||
      (p.localAddr || '').includes(search)
    )
  }

  // Update filter count
  const countEl = document.getElementById('port-filter-count')
  if (countEl) {
    const total = (state.portData.ports || []).length
    countEl.textContent = ports.length === total ? `${total}개` : `${ports.length} / ${total}`
  }

  if (!ports.length) {
    tbody.innerHTML = '<tr><td colspan="5" class="port-empty">열린 포트가 없습니다</td></tr>'
    return
  }

  tbody.innerHTML = ports.map(p => `
    <tr>
      <td class="port-num">${escapeHtml(String(p.port))}</td>
      <td>${escapeHtml(p.processName || '—')}</td>
      <td class="port-pid">${p.pid || '—'}</td>
      <td><span class="category-badge category-badge--${p.category}">${categoryLabel(p.category)}</span></td>
      <td>${p.pid ? `<button class="kill-btn" onclick="killPort(${p.pid})">종료</button>` : ''}</td>
    </tr>
  `).join('')
}

function categoryLabel(cat) {
  const labels = { web_server: 'Web', database: 'DB', development: 'Dev', system: 'Sys', other: '기타' }
  return labels[cat] || cat
}

async function killPort(pid) {
  try {
    await invoke('kill_port_process', { pid })
    await loadPorts()
  } catch(e) {
    alert(`프로세스 종료 실패: ${e}`)
  }
}

// ── Dirty-check signature ──
let _lastSig = ''
function snapshotSignature(snap) {
  const s = snap.summary
  let sig = `${s.runningAgents}|${s.suspectedStalled}`
  for (const session of snap.sessions) {
    sig += `|${session.id}:${session.agents.map(a => a.id + '=' + a.status).join(',')}`
    for (const child of (session.children || [])) {
      sig += `|c:${child.id}:${child.agents.map(a => a.id + '=' + a.status).join(',')}`
    }
  }
  return sig
}

async function loadSnapshot() {
  if (isSnapshotLoading) return
  isSnapshotLoading = true
  try {
    if (!window.__TAURI__) { return; }
    const payload = await window.__TAURI__.core.invoke('get_dashboard_snapshot');
    state.snapshot = payload;
    state.error = null;

    // Always update topbar (cheap — 3 text nodes)
    renderTopBar(payload)

    // SleepGuard — sync every poll
    if (state.snapshot && window.__TAURI__) {
      const running = state.snapshot.summary.runningAgents
      try {
        await invoke('set_sleep_prevention', {
          prevent: running > 0,
          reason: running > 0 ? 'agent_running' : 'idle',
          activeAgents: running,
        })
      } catch(_) {}
      // Always update debug UI
      try {
        const sleepStatus = await invoke('get_sleep_status')
        updateSleepUI(sleepStatus)
      } catch(_) {}
    }

    // Dirty-check: skip heavy render if snapshot is structurally identical
    const sig = snapshotSignature(payload)
    if (sig === _lastSig) {
      // Still need to keep selection valid even if no visible change
      ensureSelection(payload)
      return
    }
    _lastSig = sig

    ensureSelection(payload);
    // Full render (minus topbar which was already done)
    renderHeavy();

  } catch (error) {
    state.error = error instanceof Error ? error.message : String(error);
    if (dbStatus) { dbStatus.textContent = state.error; dbStatus.style.opacity = '1'; }
    render();
  } finally {
    isSnapshotLoading = false
    scheduleSnapshotPolling()
    schedulePortPolling()
  }
}

loadSnapshot()
initTabs()
initSleepDebug()
