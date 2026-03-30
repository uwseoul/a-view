# OpenCode Ops Dashboard

로컬 OpenCode SQLite 저장소를 읽어 세션/에이전트 상태를 보여주는 읽기 전용 운영 대시보드입니다.

## 실행

```bash
npm start
```

브라우저에서 `http://127.0.0.1:4317` 를 열면 됩니다.

## 현재 범위
- SQLite DB: `~/.local/share/opencode/opencode.db`
- 세션 / 메시지 / 파트 / todo 읽기
- 5초 polling
- stalled heuristic: 마지막 활동 45초 이상
- 읽기 전용

## 비고
- runtime BackgroundManager 메모리 상태는 이 standalone 대시보드의 1차 범위에서 제외했습니다.
- transcript 파일이 있으면 일부 최근 로그에 보조적으로 반영합니다.
