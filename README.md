# a-view

**OpenCode Ops Dashboard** — 로컬 OpenCode 세션/에이전트 상태를 실시간으로 모니터링하는 데스크톱 앱.

## 왜 a-view인가?

AI 코딩 에이전트를 여러 프로젝트에서 동시에 돌리고 있다면 — 어느 에이전트가 멈췄는지, 어느 세션이 돌아가고 있는지, 어떤 작업이 완료됐는지 일일이 터미널을 열어 확인하고 있지 않은가?

**a-view**는 그 문제를 해결한다. 하나의 앱으로 모든 OpenCode 세션의 상태를 한눈에 파악할 수 있다.

- **데스크톱 앱** — Tauri 2 기반 네이티브 앱. 가볍고 빠름
- **설정 없이 즉시 실행** — 실행만 하면 끝. 기존 OpenCode DB를 읽기만 하므로 원본에 영향 없음
- **SleepGuard** — 에이전트 작업 중 OS 절전 자동 방지. 작업이 끝나면 자동 해제
- **Port Killer** — 로컬 포트 스캔, 원클릭 프로세스 종료, 스마트 분류
- **프로젝트 단위 관리** — 여러 프로젝트를 동시에 운영해도 디렉토리별로 자동 분류
- **Stalled 자동 탐지** — 45초 무활동 에이전트를 자동 감지

## 기능

### 대시보드
- **프로젝트별 그룹핑** — 디렉토리 기준으로 세션을 프로젝트 카드로 묶어서 최근 활동순 정렬
- **에이전트 상태 추적** — Running / Stalled / Completed / Failed 실시간 표시
- **서브 에이전트 구분** — 메인 에이전트와 서브 에이전트를 시각적으로 구분
- **Stalled 탐지** — 45초 무활동 에이전트 자동 감지
- **3칼럼 독립 스크롤** — 프로젝트 사이드바, 세션/에이전트 그리드, 상세 패널
- **5초 자동 갱신** — 폴링 시 스크롤 위치 보존
- **다크모드 UI** — 모니터링에 최적화된 다크 테마

### SleepGuard (절전 방지)
- **자동 감지** — OpenCode 에이전트가 작업 중이면 OS 절전 자동 방지
- **멀티플랫폼** — Windows, macOS, Linux 모두 지원
- **상태 표시** — 상단 상태바에 실시간 절전 방지 상태 표시 (🟢 작업 중 / ⚪ 대기 중)
- **자동 해제** — 에이전트 작업 완료 시 절전 방지 자동 해제

### Port Killer (포트 관리)
- **포트 스캔** — 시스템의 모든 리스닝 TCP 포트 자동 탐지
- **스마트 분류** — Web Server, Database, Development, System, Other 자동 분류
- **원클릭 종료** — 포트를 점유한 프로세스를 즉시 종료
- **실시간 검색** — 포트 번호 또는 프로세스명으로 필터링

## 설치 및 실행

[Releases](https://github.com/uwseoul/a-view/releases) 페이지에서 OS에 맞는 설치 파일 다운로드.

> **필수 조건**: 같은 PC에 [OpenCode](https://github.com/nicepkg/opencode)가 설치되어 있고 `~/.local/share/opencode/opencode.db`가 존재해야 함

## Tech Stack

- **Desktop**: Tauri 2 (Rust)
- **Frontend**: Vanilla JS, CSS
- **Backend**: Rust (rusqlite)
- **Data source**: OpenCode SQLite DB (read-only)
- **Sleep prevention**: keepawake crate (cross-platform)
- **Port scanning**: listeners crate (cross-platform)
- **CI/CD**: GitHub Actions

## License

MIT
