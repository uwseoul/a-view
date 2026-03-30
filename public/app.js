const state = {
  snapshot: null,
  selectedDirectory: null,
  selectedSessionId: null,
  selectedAgentId: null,
  expandedProjects: new Set(),
  error: null,
}

function formatAge(ageSec) {
  if (ageSec == null) return 'n/a'
  if (ageSec < 60) return `${ageSec}s ago`
  const min = Math.floor(ageSec / 60)
  const sec = ageSec % 60
  return `${min}m ${sec}s ago`
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
  return Array.from(groups.values()).sort((a, b) => b.sessions.length - a.sessions.length)
}

function renderTopBar(snapshot) {
  document.getElementById('running-agents').textContent = snapshot.summary.runningAgents
  document.getElementById('stalled-agents').textContent = snapshot.summary.suspectedStalled
  document.getElementById('last-refreshed').textContent = `Last refresh ${new Date(snapshot.generatedAt).toLocaleTimeString()}`
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

function agentCardHtml(agent, isActive) {
  const tools = (agent.tools || []).slice(0, 4).map(t => `<span class="tool-badge">${escapeHtml(t)}</span>`).join('')
  return `
    <article class="agent-card ${isActive ? 'is-active' : ''}" data-status="${agent.status}" data-agent-id="${agent.id}">
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

    const agentsHtml = session.agents.map(a => agentCardHtml(a, a.id === state.selectedAgentId)).join('')

    let childrenAgentsHtml = ''
    if (session.children && session.children.length > 0) {
      childrenAgentsHtml = session.children.flatMap(child =>
        child.agents.map(a => agentCardHtml(a, a.id === state.selectedAgentId))
      ).join('')
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

function render() {
  if (state.error) { renderError(state.error); return }
  if (!state.snapshot) return

  const sp = {}
  const pl = document.getElementById('projects-list')
  const sm = document.getElementById('sessions-main')
  const ds = document.querySelector('.panel--detail-scroll')
  if (pl) sp.projects = pl.scrollTop
  if (sm) sp.sessions = sm.scrollTop
  if (ds) sp.detail = ds.scrollTop

  renderTopBar(state.snapshot)
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

async function loadSnapshot() {
  try {
    const response = await fetch('/api/dashboard/snapshot', { cache: 'no-store' })
    const payload = await response.json()
    if (!response.ok) throw new Error(payload.message || 'snapshot load failed')
    state.snapshot = payload
    state.error = null
    ensureSelection(payload)
    render()
  } catch (error) {
    state.error = error instanceof Error ? error.message : String(error)
    render()
  }
}

loadSnapshot()
setInterval(loadSnapshot, 5000)
