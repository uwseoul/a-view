# a-view

**OpenCode Ops Dashboard** — 로컬 OpenCode 세션/에이전트 상태를 실시간으로 모니터링하는 웹 대시보드.

## 왜 a-view인가?

AI 코딩 에이전트를 여러 프로젝트에서 동시에 돌리고 있다면 — 어느 에이전트가 멈췄는지, 어느 세션이 돌아가고 있는지, 어떤 작업이 완료됐는지 일일이 터미널을 열어 확인하고 있지 않은가?

**a-view**는 그 문제를 해결한다. 브라우저 하나로 모든 OpenCode 세션의 상태를 한눈에 파악할 수 있다.

- **설정 없이 즉시 실행** — `npm start` 한 번이면 끝. 기존 OpenCode DB를 읽기만 하므로 원본에 영향 없음
- **제로 의존성** — React, Vue, Webpack 없이 Vanilla JS + Node.js 기본 모듈만으로 동작. `npm install`조차 필요 없다
- **프로젝트 단위 관리** — 여러 프로젝트를 동시에 운영해도 디렉토리별로 자동 분류. 최근 활동순 정렬로 핫한 프로젝트부터 확인
- **Stalled 자동 탐지** — 45초 무활동 에이전트를 자동 감지해서 방치된 작업을 놓치지 않음
- **어디서든 접속** — Tailscale이나 Cloudflare Tunnel로 외부에서도 모바일로 확인 가능

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

브라우저에서 `http://localhost:4317` 열기. 끝.

> **필수 조건**: [OpenCode](https://github.com/nicepkg/opencode)가 설치되어 있고 `~/.local/share/opencode/opencode.db`가 존재해야 함

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
- **Data source**: OpenCode SQLite DB (read-only)
- **Polling**: 5초 간격

## Roadmap

- 웹 터미널 (xterm.js + WebSocket)
- 원격 접속 (Tailscale / Cloudflare Tunnel)
- 에이전트 제어 (프롬프트 전송, 모델 전환)

## License

MIT
