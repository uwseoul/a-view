const { readOpenCodeSessions, DEFAULT_DB_PATH } = require('./opencode-adapter')
const { ageSecFrom, classifyStatus } = require('./stall-detector')

function titleCaseStatus(status) {
  return status.charAt(0).toUpperCase() + status.slice(1)
}

function sortLogs(logs) {
  return [...logs].sort((a, b) => new Date(b.time) - new Date(a.time)).slice(0, 20)
}

function nestSessions(flatSessions) {
  const sessionMap = new Map()
  for (const session of flatSessions) {
    sessionMap.set(session.id, { ...session, children: [] })
  }

  const roots = []
  for (const session of sessionMap.values()) {
    if (session.parentId && sessionMap.has(session.parentId)) {
      sessionMap.get(session.parentId).children.push(session)
    } else {
      roots.push(session)
    }
  }

  for (const root of roots) {
    root.children.sort((a, b) => {
      const aTime = new Date(a.lastActivityAt || 0).getTime()
      const bTime = new Date(b.lastActivityAt || 0).getTime()
      return bTime - aTime
    })
  }

  return roots
}

function buildSnapshot({ now = Date.now(), limit = 20, dbPath = DEFAULT_DB_PATH } = {}) {
  const flatSessions = readOpenCodeSessions({ limit, dbPath })

  const sessions = nestSessions(flatSessions).map((session) => {
    const agents = session.agents.map((agent) => {
      const status = classifyStatus({
        explicitStatus: agent.explicitStatus,
        hasRunningTool: agent.hasRunningTool,
        lastActivityAt: agent.lastActivityAt,
        now,
      })

      const recentLogs = sortLogs(agent.recentLogs).map((log) => ({
        time: log.time,
        level: log.level,
        message: log.message,
      }))

      const latestLog = recentLogs[0]
      const task = agent.task || latestLog?.message || session.todoSummary || '활동 요약 없음'
      return {
        id: agent.id,
        name: agent.name,
        model: agent.model,
        status,
        statusLabel: titleCaseStatus(status),
        task,
        startedAt: agent.startedAt,
        lastActivityAt: agent.lastActivityAt,
        durationSec: agent.startedAt ? Math.max(0, Math.floor((now - new Date(agent.startedAt).getTime()) / 1000)) : null,
        lastEventAgeSec: ageSecFrom(agent.lastActivityAt, now),
        isStalled: status === 'stalled',
        tools: Array.from(agent.tools).slice(0, 4),
        recentLogs,
      }
    }).sort((a, b) => {
      const order = { stalled: 0, delayed: 1, running: 2, failed: 3, completed: 4 }
      return (order[a.status] ?? 99) - (order[b.status] ?? 99)
    })

    const statusCounts = { running: 0, delayed: 0, stalled: 0, completed: 0, failed: 0 }
    for (const agent of agents) statusCounts[agent.status] += 1

    const children = (session.children || []).map((child) => ({
      id: child.id,
      name: child.name,
      agents: child.agents.map((agent) => {
        const status = classifyStatus({
          explicitStatus: agent.explicitStatus,
          hasRunningTool: agent.hasRunningTool,
          lastActivityAt: agent.lastActivityAt,
          now,
        })
        const recentLogs = sortLogs(agent.recentLogs).map((log) => ({
          time: log.time,
          level: log.level,
          message: log.message,
        }))
        return {
          id: agent.id,
          name: agent.name,
          model: agent.model,
          status,
          statusLabel: titleCaseStatus(status),
          task: agent.task || recentLogs[0]?.message || '활동 요약 없음',
          startedAt: agent.startedAt,
          lastActivityAt: agent.lastActivityAt,
          lastEventAgeSec: ageSecFrom(agent.lastActivityAt, now),
          isStalled: status === 'stalled',
          tools: Array.from(agent.tools).slice(0, 4),
          recentLogs,
        }
      }),
    }))

    return {
      id: session.id,
      name: session.name,
      directory: session.directory,
      startedAt: session.startedAt,
      lastActivityAt: session.lastActivityAt,
      durationSec: session.startedAt ? Math.max(0, Math.floor((now - new Date(session.startedAt).getTime()) / 1000)) : null,
      stalledAgentCount: statusCounts.stalled,
      statusCounts,
      agents,
      children,
    }
  })

  const summary = sessions.reduce((acc, session) => {
    acc.runningAgents += session.statusCounts.running
    acc.suspectedStalled += session.statusCounts.stalled
    acc.totalSessions += 1
    return acc
  }, { runningAgents: 0, suspectedStalled: 0, totalSessions: 0 })

  return {
    generatedAt: new Date(now).toISOString(),
    source: {
      dbPath: DEFAULT_DB_PATH,
      mode: 'sqlite',
      refreshIntervalSec: 5,
      stalledThresholdSec: 45,
    },
    summary,
    sessions,
  }
}

module.exports = { buildSnapshot }
