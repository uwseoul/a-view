const fs = require('node:fs')
const path = require('node:path')
const os = require('node:os')
const { DatabaseSync } = require('node:sqlite')

const DEFAULT_DB_PATH = path.join(os.homedir(), '.local', 'share', 'opencode', 'opencode.db')
const DEFAULT_TRANSCRIPTS_DIR = path.join(os.homedir(), '.claude', 'transcripts')
const SYSTEM_AGENT_NAMES = new Set(['compaction', 'session'])

function safeJsonParse(value) {
  if (!value || typeof value !== 'string') return null
  try {
    return JSON.parse(value)
  } catch {
    return null
  }
}

function asIsoFromMs(ms) {
  if (!Number.isFinite(ms)) return null
  return new Date(ms).toISOString()
}

function firstNonEmpty(...values) {
  for (const value of values) {
    if (typeof value === 'string' && value.trim()) return value.trim()
  }
  return null
}

function pickAgentName(messageJson) {
  return firstNonEmpty(messageJson?.agent, messageJson?.mode, messageJson?.model?.modelID, 'session')
}

function pickModel(messageJson) {
  return firstNonEmpty(
    messageJson?.model?.providerID && messageJson?.model?.modelID
      ? `${messageJson.model.providerID}/${messageJson.model.modelID}`
      : null,
    messageJson?.providerID && messageJson?.modelID
      ? `${messageJson.providerID}/${messageJson.modelID}`
      : null,
    messageJson?.modelID,
    'unknown',
  )
}

function summarizePart(partJson) {
  if (!partJson || typeof partJson !== 'object') return null
  const type = partJson.type
  if (type === 'text') return firstNonEmpty(partJson.text)
  if (type === 'tool') {
    const tool = firstNonEmpty(partJson.tool, 'tool')
    const status = firstNonEmpty(partJson.state?.status)
    const input = partJson.state?.input?.command
    return firstNonEmpty([tool, status].filter(Boolean).join(' · '), input)
  }
  if (type === 'step-start') return '작업 단계 시작'
  if (type === 'step-finish') return '작업 단계 완료'
  if (type === 'reasoning') return 'Reasoning update'
  return firstNonEmpty(partJson.text, partJson.tool)
}

function deriveLogLevel(partJson, messageJson) {
  const explicit = firstNonEmpty(partJson?.state?.status)
  if (explicit === 'failed' || explicit === 'error') return 'error'
  if (explicit === 'completed' || explicit === 'success') return 'success'
  if (explicit === 'pending') return 'warn'
  if (partJson?.type === 'tool') return 'tool'
  if (messageJson?.role === 'assistant') return 'info'
  return 'info'
}

function readTranscriptTail(sessionId, limit = 8) {
  const filePath = path.join(DEFAULT_TRANSCRIPTS_DIR, `${sessionId}.jsonl`)
  if (!fs.existsSync(filePath)) return []
  const content = fs.readFileSync(filePath, 'utf8').trim()
  if (!content) return []
  return content
    .split('\n')
    .slice(-limit)
    .map((line) => safeJsonParse(line))
    .filter(Boolean)
    .map((row) => ({
      agentName: firstNonEmpty(row.agent, row.mode, 'session'),
      time: asIsoFromMs(row.time?.created ?? row.time_created ?? row.timestamp ?? Date.now()),
      message: firstNonEmpty(row.text, row.message, row.summary, row.type),
      level: row.type === 'error' ? 'error' : 'info',
    }))
}

function openDb(dbPath = DEFAULT_DB_PATH) {
  if (!fs.existsSync(dbPath)) {
    throw new Error(`OpenCode DB not found: ${dbPath}`)
  }
  return new DatabaseSync(dbPath, { readonly: true })
}

function readRawSessions({ limit = 20, dbPath = DEFAULT_DB_PATH } = {}) {
  const db = openDb(dbPath)
  try {
    const sessions = db.prepare(`
      SELECT id, title, directory, parent_id, time_created, time_updated
      FROM session
      WHERE time_archived IS NULL
      ORDER BY time_updated DESC
      LIMIT ?
    `).all(limit)

    return sessions.map((session) => {
      const messages = db.prepare(`
        SELECT id, session_id, time_created, time_updated, data
        FROM message
        WHERE session_id = ?
        ORDER BY time_updated DESC
        LIMIT 80
      `).all(session.id)

      const parts = db.prepare(`
        SELECT p.id, p.message_id, p.session_id, p.time_created, p.time_updated, p.data
        FROM part p
        WHERE p.session_id = ?
        ORDER BY p.time_updated DESC
        LIMIT 160
      `).all(session.id)

      const todos = db.prepare(`
        SELECT content, status, priority, position, time_created, time_updated
        FROM todo
        WHERE session_id = ?
        ORDER BY position ASC
      `).all(session.id)

      return { session, messages, parts, todos, transcriptTail: readTranscriptTail(session.id) }
    })
  } finally {
    db.close()
  }
}

function normalizeRawSession(rawSession) {
  const messageMap = new Map()
  for (const messageRow of rawSession.messages) {
    const messageJson = safeJsonParse(messageRow.data) || {}
    messageMap.set(messageRow.id, { ...messageRow, json: messageJson })
  }

  const agentBuckets = new Map()
  const ensureAgent = (agentName, model, createdAt) => {
    const id = `${rawSession.session.id}:${agentName}`
    if (!agentBuckets.has(id)) {
      agentBuckets.set(id, {
        id,
        name: agentName,
        model: model || 'unknown',
        startedAt: createdAt || null,
        lastActivityAt: null,
        explicitStatus: null,
        hasRunningTool: false,
        task: null,
        tools: new Set(),
        recentLogs: [],
      })
    }
    const agent = agentBuckets.get(id)
    if (model && agent.model === 'unknown') agent.model = model
    if (!agent.startedAt && createdAt) agent.startedAt = createdAt
    return agent
  }

  const sortedMessages = [...rawSession.messages].sort((a, b) => a.time_created - b.time_created)

  for (const messageRow of sortedMessages) {
    const messageJson = safeJsonParse(messageRow.data) || {}
    const agentName = pickAgentName(messageJson)
    const model = pickModel(messageJson)
    const createdAt = asIsoFromMs(messageRow.time_created)
    const updatedAt = asIsoFromMs(messageRow.time_updated)
    const agent = ensureAgent(agentName, model, createdAt)
    if (!agent.lastActivityAt || new Date(updatedAt) > new Date(agent.lastActivityAt)) {
      agent.lastActivityAt = updatedAt
    }
    if (!agent.task && messageJson.role === 'user') {
      const content = firstNonEmpty(messageJson.content, messageJson.summary?.text, messageJson.summary)
      if (content) agent.task = String(content).slice(0, 120)
    }
    if (!agent.task) {
      agent.task = firstNonEmpty(messageJson?.summary?.text, messageJson?.summary, messageJson?.title)
    }
  }

  for (const partRow of rawSession.parts) {
    const partJson = safeJsonParse(partRow.data) || {}
    const message = messageMap.get(partRow.message_id)
    const messageJson = message?.json || {}
    const agentName = pickAgentName(messageJson)
    const model = pickModel(messageJson)
    const updatedAt = asIsoFromMs(partRow.time_updated)
    const agent = ensureAgent(agentName, model, asIsoFromMs(partRow.time_created))
    if (!agent.lastActivityAt || new Date(updatedAt) > new Date(agent.lastActivityAt)) {
      agent.lastActivityAt = updatedAt
    }
    const toolName = firstNonEmpty(partJson?.tool)
    if (toolName) agent.tools.add(toolName)

    const toolStatus = firstNonEmpty(partJson?.state?.status)
    if (toolStatus === 'running') agent.hasRunningTool = true

    if (toolStatus === 'running' && partJson?.state?.input?.command) {
      agent.task = `[${toolName}] ${partJson.state.input.command}`.slice(0, 120)
    }

    const logMessage = summarizePart(partJson)
    if (logMessage) {
      agent.recentLogs.push({
        time: updatedAt,
        level: deriveLogLevel(partJson, messageJson),
        message: logMessage,
      })
      if (!agent.task && partJson.type !== 'reasoning') {
        agent.task = logMessage
      }
    }

    if (toolStatus) agent.explicitStatus = toolStatus
  }

  for (const transcriptRow of rawSession.transcriptTail) {
    const agent = ensureAgent(transcriptRow.agentName, 'unknown', transcriptRow.time)
    agent.recentLogs.push({
      time: transcriptRow.time,
      level: transcriptRow.level,
      message: transcriptRow.message,
    })
    if (!agent.lastActivityAt || new Date(transcriptRow.time) > new Date(agent.lastActivityAt)) {
      agent.lastActivityAt = transcriptRow.time
    }
    if (!agent.task) agent.task = transcriptRow.message
  }

  const todoSummary = rawSession.todos.length
    ? `Todo ${rawSession.todos.filter((todo) => todo.status === 'completed').length}/${rawSession.todos.length}`
    : null

  let agents = Array.from(agentBuckets.values())
  if (agents.length > 1) {
    agents = agents.filter((a) => !SYSTEM_AGENT_NAMES.has(a.name))
  }
  if (agents.length === 0) {
    const fallback = ensureAgent('session', 'unknown', asIsoFromMs(rawSession.session.time_created))
    fallback.lastActivityAt = asIsoFromMs(rawSession.session.time_updated)
    fallback.task = todoSummary || '세션 활동 없음'
    agents = [fallback]
  }

  return {
    id: rawSession.session.id,
    parentId: rawSession.session.parent_id || null,
    name: rawSession.session.title,
    directory: rawSession.session.directory,
    startedAt: asIsoFromMs(rawSession.session.time_created),
    lastActivityAt: asIsoFromMs(rawSession.session.time_updated),
    todos: rawSession.todos,
    todoSummary,
    agents,
  }
}

function readOpenCodeSessions(options) {
  return readRawSessions(options).map(normalizeRawSession)
}

module.exports = {
  DEFAULT_DB_PATH,
  readOpenCodeSessions,
}
