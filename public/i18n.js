// i18n — Internationalization module for A-view
// Supports: ko (Korean), en (English)

const translations = {
  ko: {
    // Brand
    brandSubtitle: 'OpenCode Ops Dashboard',

    // Tabs
    tabDashboard: 'Dashboard',
    tabPortKiller: 'Port Killer',

    // KPI
    runningAgents: '실행 중인 에이전트',
    suspectedStalled: '지연 의심',

    // Time
    justNow: '방금 전',
    secondsAgo: '초 전',
    minutesAgo: '분 전',
    hoursAgo: '시간 전',
    daysAgo: '일 전',

    // Duration
    hours: '시간',
    minutes: '분',
    seconds: '초',

    // Projects
    projects: 'Projects',
    projectCount: '{count}개 프로젝트',
    sessions: 'sessions',
    running: 'running',
    stalled: 'stalled',

    // Agent status
    statusRunning: 'Running',
    statusDelayed: 'Delayed',
    statusStalled: 'Stalled',
    statusFailed: 'Failed',
    statusCompleted: 'Completed',

    // Agent detail
    lastActivity: 'Last activity',
    duration: 'Duration',
    started: 'Started',
    tools: 'Tools',
    logs: 'Logs',
    showAll: '전체 보기 {count}',
    showLess: '접기',
    selectAgent: '에이전트를 선택하면 상세가 표시됩니다.',
    noRecentLogs: '최근 로그가 없습니다.',
    noActivity: '활동 요약 없음',
    noSessionActivity: '세션 활동 없음',

    // SleepGuard
    sleepPreventing: '절전 방지',
    sleepSystem: '시스템 설정에 따름',
    sleepGuard: 'SleepGuard',
    sleepStatus: '상태',
    sleepMode: '모드',
    sleepActiveAgents: '활성 에이전트',
    sleepDisplay: '화면 꺼짐 방지',
    sleepIdle: '시스템 대기 방지',
    sleepStartTime: '시작 시각',
    sleepReleaseTime: '해제 시각',
    sleepTime: '시각',
    sleepDuration: '유지 시간',
    sleepElapsed: '경과 시간',
    sleepTimeLabel: '시간',
    sleepNextCheck: '다음 체크',
    sleepExpect: '예상',
    sleepModeActive: '시스템 설정 무시 · 절전 차단',
    sleepModeIdle: 'OS 기본 절전 정책 따름',
    sleepExpectOff: '수동 끄기 → 시스템 설정에 따름',
    sleepExpectOn: '수동 켜기 → 절전 방지',
    sleepManualOff: '수동 끄기',
    sleepManualOn: '수동 켜기',
    sleepCountdown: '{sec}초 후 상태 확인',
    sleepChecking: '확인 중...',
    sleepPreventingLabel: '🟢 절전 방지 중',
    sleepSystemLabel: '⚪ 시스템 설정에 따름',

    // Polling
    polling: '{sec}s 간격 폴링',

    // Port Killer
    portSearch: '포트 또는 프로세스 검색...',
    portRefresh: '새로고침',
    portCategoryFilter: '분류 필터',
    portAll: '전체',
    portOther: '기타',
    portColumnPort: '포트',
    portColumnProcess: '프로세스',
    portColumnPid: 'PID',
    portColumnCategory: '분류',
    portColumnAction: '동작',
    portKill: '종료',
    portKillSuccess: '프로세스(PID: {pid})가 종료되었습니다.',
    portScanning: '포트를 스캔하는 중...',
    portScanFail: '포트 스캔 실패: {error}',
    portEmpty: '열린 포트가 없습니다',
    portKillFail: '프로세스 종료 실패: {error}',
    portCount: '{count}개',
    portCountFiltered: '{filtered} / {total}',
  },

  en: {
    // Brand
    brandSubtitle: 'OpenCode Ops Dashboard',

    // Tabs
    tabDashboard: 'Dashboard',
    tabPortKiller: 'Port Killer',

    // KPI
    runningAgents: 'Running Agents',
    suspectedStalled: 'Suspected Stalled',

    // Time
    justNow: 'just now',
    secondsAgo: 's ago',
    minutesAgo: 'm ago',
    hoursAgo: 'h ago',
    daysAgo: 'd ago',

    // Duration
    hours: 'h',
    minutes: 'm',
    seconds: 's',

    // Projects
    projects: 'Projects',
    projectCount: '{count} projects',
    sessions: 'sessions',
    running: 'running',
    stalled: 'stalled',

    // Agent status
    statusRunning: 'Running',
    statusDelayed: 'Delayed',
    statusStalled: 'Stalled',
    statusFailed: 'Failed',
    statusCompleted: 'Completed',

    // Agent detail
    lastActivity: 'Last activity',
    duration: 'Duration',
    started: 'Started',
    tools: 'Tools',
    logs: 'Logs',
    showAll: 'Show all {count}',
    showLess: 'Show less',
    selectAgent: 'Select an agent to see details.',
    noRecentLogs: 'No recent logs.',
    noActivity: 'No activity summary',
    noSessionActivity: 'No session activity',

    // SleepGuard
    sleepPreventing: 'Sleep Prevention',
    sleepSystem: 'Follow System Settings',
    sleepGuard: 'SleepGuard',
    sleepStatus: 'Status',
    sleepMode: 'Mode',
    sleepActiveAgents: 'Active Agents',
    sleepDisplay: 'Display Sleep Block',
    sleepIdle: 'System Idle Block',
    sleepStartTime: 'Started At',
    sleepReleaseTime: 'Released At',
    sleepTime: 'Time',
    sleepDuration: 'Duration',
    sleepElapsed: 'Elapsed',
    sleepTimeLabel: 'Time',
    sleepNextCheck: 'Next Check',
    sleepExpect: 'Expect',
    sleepModeActive: 'Ignore system settings · Block sleep',
    sleepModeIdle: 'Follow OS default sleep policy',
    sleepExpectOff: 'Turn off → Follow system settings',
    sleepExpectOn: 'Turn on → Prevent sleep',
    sleepManualOff: 'Turn Off',
    sleepManualOn: 'Turn On',
    sleepCountdown: 'Check in {sec}s',
    sleepChecking: 'Checking...',
    sleepPreventingLabel: '🟢 Preventing Sleep',
    sleepSystemLabel: '⚪ Follow System Settings',

    // Polling
    polling: 'Polling every {sec}s',

    // Port Killer
    portSearch: 'Search port or process...',
    portRefresh: 'Refresh',
    portCategoryFilter: 'Category Filter',
    portAll: 'All',
    portOther: 'Other',
    portColumnPort: 'Port',
    portColumnProcess: 'Process',
    portColumnPid: 'PID',
    portColumnCategory: 'Category',
    portColumnAction: 'Action',
    portKill: 'Kill',
    portKillSuccess: 'Process (PID: {pid}) has been terminated.',
    portScanning: 'Scanning ports...',
    portScanFail: 'Port scan failed: {error}',
    portEmpty: 'No open ports',
    portKillFail: 'Process kill failed: {error}',
    portCount: '{count}',
    portCountFiltered: '{filtered} / {total}',
  }
}

// Current language
let _lang = localStorage.getItem('a-view-lang') || 'ko'

function t(key, params) {
  const dict = translations[_lang] || translations.ko
  let str = dict[key] || translations.ko[key] || key
  if (params) {
    for (const [k, v] of Object.entries(params)) {
      str = str.replace(`{${k}}`, v)
    }
  }
  return str
}

function getLang() {
  return _lang
}

function setLang(lang) {
  if (!translations[lang]) return
  _lang = lang
  localStorage.setItem('a-view-lang', lang)
}

function toggleLang() {
  setLang(_lang === 'ko' ? 'en' : 'ko')
}
