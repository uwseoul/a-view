const RUNNING_THRESHOLD_SEC = 30
const STALLED_THRESHOLD_SEC = 45

function clampAgeSec(value) {
  return Number.isFinite(value) && value >= 0 ? Math.floor(value) : null
}

function ageSecFrom(lastActivityAt, now = Date.now()) {
  if (!lastActivityAt) return null
  const ts = new Date(lastActivityAt).getTime()
  if (!Number.isFinite(ts)) return null
  return clampAgeSec((now - ts) / 1000)
}

function classifyStatus({ explicitStatus, hasRunningTool, lastActivityAt, now = Date.now() }) {
  const norm = typeof explicitStatus === 'string' ? explicitStatus.toLowerCase() : null

  if (norm === 'failed' || norm === 'error') return 'failed'
  if (norm === 'running') return 'running'
  if (norm === 'completed' || norm === 'success' || norm === 'done') {
    return hasRunningTool ? 'running' : 'completed'
  }

  const ageSec = ageSecFrom(lastActivityAt, now)
  if (ageSec == null) return hasRunningTool ? 'running' : 'delayed'
  if (ageSec < RUNNING_THRESHOLD_SEC) return 'running'
  if (ageSec < STALLED_THRESHOLD_SEC) return 'delayed'
  return 'stalled'
}

module.exports = {
  RUNNING_THRESHOLD_SEC,
  STALLED_THRESHOLD_SEC,
  ageSecFrom,
  classifyStatus,
}
