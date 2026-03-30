# a-view

**OpenCode Ops Dashboard** — 로컬 OpenCode 세션/에이전트 상태를 실시간으로 모니터링하는 웹 대시보드.

## Features

- **프로젝트별 그룹핑** — 디렉토리 기준으로 세션을 프로젝트 카드로 묶어서 최근 활동순 정렬
- **에이전트 상태 추적** — Running / Stalled / Completed / Failed 실시간 표시
- **Stalled 탐지** — 45초 무활동 에이전트 자동 감지
- **3칼럼 독립 스크롤** — 프로젝트 사이드바, 세션/에이전트 그리드, 상세 패널 각각 스크롤 유지
- **5초 자동 갱신** — 폴링 시 스크롤 위치 보존
- **다크모드 UI** — 모니터링에 최적화된 다크 테마

## Quick Start

```bash
git clone https://github.com/uwseoul/a-view.git
cd a-view
npm start
```

브라우저에서 `http://localhost:4317` 열기.

## Architecture

```
public/
  index.html     SPA 엔트리
  styles.css     다크 테마 스타일시트
  app.js         프론트엔드 렌더링 로직

server/
  server.js             HTTP 서버 (정적 파일 + API)
  dashboard-service.js  스냅샷 빌더
  opencode-adapter.js   SQLite 어댑터 (세션/메시지/파트 읽기)
  stall-detector.js     Stalled 탐지 로직

test/
  stall-detector.test.js
```

## Tech Stack

- **Runtime**: Node.js (built-in `node:sqlite`)
- **Frontend**: Vanilla JS, CSS (no framework)
- **Data source**: OpenCode SQLite DB (`~/.local/share/opencode/opencode.db`)
- **Polling**: 5초 간격

## Roadmap

- 웹 터미널 (xterm.js + WebSocket)
- 원격 접속 (Tailscale / Cloudflare Tunnel)
- 에이전트 제어 (프롬프트 전송, 모델 전환)

## License

MIT
