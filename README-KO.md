# AxAgent

[![Release](https://img.shields.io/github/v/release/polite0803/AxAgent?style=flat-square)](https://github.com/polite0803/AxAgent/releases)
[![Build](https://img.shields.io/github/actions/workflow/status/polite0803/AxAgent/release.yml?style=flat-square)](https://github.com/polite0803/AxAgent/actions)
![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux%20%7C%20Android%20%7C%20iOS-blue?style=flat-square)
![License](https://img.shields.io/badge/license-AGPL--3.0-green?style=flat-square)

<p align="center">
  <a href="./media/poster-axagent.svg">
    <img src="./media/poster-axagent.svg" alt="AxAgent 포스터" width="80%" />
  </a>
</p>

**AxAgent**는 Tauri 2 기반의 크로스플랫폼 AI 어시스턴트 데스크톱 클라이언트입니다(Windows / macOS / Linux / Android / iOS). ReAct 에이전트 엔진, 비주얼 워크플로우 오케스트레이션, 로컬 RAG 지식 베이스, MCP 프로토콜 확장, 통합 멀티 모델 게이트웨이, 브라우저 자동화, 컴퓨터 제어를 통합하여 일상적인 개발, 연구, 지식 관리 및 자동화를 위한 AI 워크스테이션입니다.

> **언어**: [简体中文](./README.md) | [English](./README-EN.md) | [繁體中文](./README-ZH-TW.md) | [日本語](./README-JA.md) | [한국어](./README-KO.md) | [Français](./README-FR.md) | [Deutsch](./README-DE.md) | [Español](./README-ES.md) | [Русский](./README-RU.md) | [हिन्दी](./README-HI.md) | [العربية](./README-AR.md)

---

## 프로젝트 포지셔닝

AxAgent는 세 가지 핵심 문제를 해결합니다:

1. **통합 멀티 모델 액세스 및 지능형 라우팅** — 단일 인터페이스에서 OpenAI, Anthropic Claude, Google Gemini, Ollama 로컬 모델 및 모든 OpenAI 호환 API를 사용하며, 멀티 키 할당량 자동 로테이션, 작업 유형별 지능형 라우팅, 스트리밍 비교 지원
2. **AI의 대화에서 실행까지 폐쇄 루프** — 47+ 내장 도구 + 비주얼 워크플로우 + MCP 확장 + 브라우저/컴퓨터 제어, AI가 파일 조작, 코드 실행, Git 관리, 작업 스케줄링 가능
3. **로컬 우선 데이터 주권** — 대화, 지식 베이스, 메모리, 설정은 모두 로컬 SQLite 데이터베이스에 저장되며, API 키는 AES-256-GCM으로 암호화. 서드파티 클라우드 서비스 없이 핵심 기능 작동

---

## 핵심 기능

### 멀티 모델 엔진

- **9개 제공자 어댑터**: OpenAI (Chat Completions + Responses + Realtime), Anthropic Claude, Google Gemini, Ollama (GGUF 로컬 모델 관리 포함), OpenClaw, Hermes 및 모든 OpenAI 호환 API
- **멀티 키 로테이션**: 동일 제공자의 여러 API 키, 할당량 기반 자동 로테이션, 단일 키 제한 시 자동 페일오버
- **지능형 라우팅**: 작업 유형(코드 리뷰 / 요약 / 번역 / 일반)에 따른 자동 모델 선택, 사용자 정의 규칙 지원
- **제공자 상태 모니터링**: 성공률, 지연 시간, 가용성 실시간 추적, 단계적 자동 폴백
- **AI 이미지 생성**: DALL-E 3 및 Flux (Replicate) 멀티 사이즈 프리셋
- **실시간 음성**: OpenAI Realtime API 기반 WebSocket 음성 대화, 중단 및 스트리밍 트랜스크립션 지원

### 에이전트 시스템 (ReAct 엔진)

- **계층적 플래너** (`hierarchical_planner`): 복잡한 작업을 Phase → Task 구조화된 계획으로 분해, DAG 토폴로지 실행으로 컴파일
- **심층 리서치** (`deep_research`): 멀티 소스 검색 오케스트레이션(검색 계획, 실행, 콘텐츠 통합, 인용 추적)
- **팩트 체커** (`fact_checker`): AI 기반 사실 검증, 소스 분류기 및 신뢰성 평가 포함
- **생각의 나무** (`tree_of_thoughts`): 다중 경로 추론 탐색, 분기 평가 및 백트래킹
- **리플렉터** (`reflector`): 실행 후 자체 평가 및 개선 제안
- **자체 검증** (`self_verifier`): 추론 결과 자동 검증, 순환 감지 포함
- **오류 복구** (`error_recovery_engine`): 오류 유형 분류 → 복구 전략 선택 → 자동 재시도 또는 계획 조정, 지수 백오프 지원
- **A/B 테스트** (`ab_testing`): 다양한 추론 전략 비교 평가
- **평가 시스템** (`evaluator`): 내장 벤치마크 프레임워크
- **LoRA 파인튜닝** (`fine_tune`): 내장 학습 파이프라인, LoRA 어댑터 관리
- **RL 옵티마이저** (`rl_optimizer`): 경험 피드백 기반 정책 강화 학습

**멀티 에이전트 협업**:

- 마스터-슬레이브 협업 아키텍처, 서브 에이전트 병렬 실행, 의존성 인식 스케줄링
- 에이전트 간 정보 교환을 위한 공유 블랙보드
- 적대적 토론 모드(Pro/Con 라운드 및 논점 강도 점수)
- 멀티 프로세스 에이전트 클러스터의 Swarm 모드
- 능동 모드: 에이전트가 자발적으로 제안 및 작업 시작 가능

**컴퓨터 제어**: AI 기반 마우스 클릭, 키보드 입력, 화면 스크롤. 3단계 권한(기본/편집 수락/전체 액세스), 샌드박스 경로 격리

**브라우저 자동화**: CDP 프로토콜을 통한 브라우저 제어, 탐색, 스크린샷, 클릭, 양식 작성, 텍스트 추출 지원

### 스킬 시스템

- **스킬 마켓플레이스**: 커뮤니티 스킬 탐색 및 설치
- **AI 지원 생성**: 자연어 제안에서 스킬 구조 자동 생성 (`skill:create`)
- **스킬 진화** (`evolution_engine`): 실행 피드백 기반 스킬 자동 분석 및 개선
- **의미적 매칭**: 컨텍스트 기반 의미적 스킬 추천
- **스킬 분해** (`skill_decomposition`): 복잡한 작업을 원자적 스킬 조합으로 자동 분해
- **생성 도구**: AI가 생성하고 등록하는 새로운 도구
- **샌드박스 실행**: 스킬은 격리된 샌드박스에서 안전하게 실행

### 비주얼 워크플로우

ReactFlow 12 기반의 드래그 앤 드롭 DAG 워크플로우 편집기:

- **17가지 노드 유형**: 트리거, 에이전트, LLM 호출, 조건 분기, 병렬 포크, 루프, 병합, 지연, 도구 호출, 코드 실행, 서브 워크플로우, 벡터 검색, 문서 파싱, 검증, 종료, 비즈니스 규칙, 에이전트 역할
- **Kahn 토폴로지 정렬 실행**: 자동 순환 의존성 감지, 병렬 파이프라인 스케줄링
- **내장 템플릿**: 코드 리뷰, 버그 수정, 문서화, 테스트, 리팩토링, 탐색, 성능 분석, 보안 감사, 기능 개발
- **YAML 직렬화**: 워크플로우 정의 가져오기/내보내기
- **버전 관리**: 템플릿 버전 관리
- **AI 보조 설계**: AI 보조 워크플로우 설계 및 노드 추천

### 지식 관리

- **멀티 지식 베이스 RAG**: 문서 업로드 → 자동 파싱(PDF/DOCX/XLSX/PPTX/TXT) → 청킹 → 벡터 인덱싱
- **하이브리드 검색**: 벡터 유사도(sqlite-vec + candle 로컬 임베딩) + BM25 전문 검색(FTS5), 하이브리드 랭킹
- **Self-RAG**: 검색 결과 자동 리플렉션 및 검증
- **리랭킹**: Cross-encoder 결과 리랭킹
- **지식 그래프**: 엔티티 추출 → 관계 구축 → 비주얼 그래프
- **파일 감시**: `notify` 기반 실시간 파일 변경 감시, 자동 증분 인덱싱
- **LLM Wiki**: AI 보조 Wiki 컴파일러 및 검증기

### 메모리 시스템

- **멀티 네임스페이스 메모리**: 프로젝트/주제별 격리, 수동 입력 및 AI 자동 추출 지원
- **영속적 통합**: Honcho 및 Mem0 폐쇄 루프 메모리
- **사용자 프로필**: 코딩 스타일, 기술 스택 선호도, 커뮤니케이션 스타일 자동 학습
- **스타일 전이**: 코드 스타일 특징 추출 → AI 생성 코드에 적용
- **드림 통합**: 메모리 조각과 행동 패턴의 백그라운드 자동 통합, 구조화된 지식 생성
- **프로젝트 메모리**: 프로젝트별 컨텍스트 영속화

### API 게이트웨이

`axum` 기반 HTTP + WebSocket 게이트웨이 내장:

- **호환 엔드포인트**: OpenAI `/v1/chat/completions`, Claude Messages API, Gemini API 및 OpenAI Responses와 Realtime WebSocket
- **키 관리**: 액세스 키 생성, 취소, 활성화/비활성화, 만료 시간 지원
- **사용량 추적**: 키/제공자/날짜별 요청 수 및 토큰 소비 통계, Prometheus 메트릭 내보내기
- **속도 제한**: `governor` 기반 토큰 버킷 알고리즘
- **SSL/TLS**: 내장 자체 서명 인증서(`rcgen`), 사용자 정의 인증서 지원
- **외부 연결**: Claude CLI, OpenCode 등 외부 도구와 원클릭 통합, API 키 자동 동기화
- **실시간 티켓**: HMAC 기반 임시 인증 티켓, WebSocket 연결 안전한 전달

### 메시징 플랫폼 통합

`rt-messaging`을 통한 멀티 플랫폼 게이트웨이. **DingTalk, Feishu, QQ, Slack, WeChat, WhatsApp, Telegram, Discord**의 메시지 수신, 명령 파싱, AI 자동 응답 지원.

### 도구 시스템

47+ 내장 도구, `Tool` trait으로 통일 등록:

| 카테고리     | 도구                                                                                                                                                                                                       |
| ------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 파일 작업    | `file_read`, `file_write`, `file_edit`, `file_system`                                                                                                                                                      |
| 코드 실행    | `bash`, `repl`                                                                                                                                                                                             |
| 검색         | `grep`, `glob`                                                                                                                                                                                             |
| 브라우저     | `browser` (CDP)                                                                                                                                                                                            |
| 컴퓨터 제어  | `computer_use` (마우스/키보드/스크린샷)                                                                                                                                                                    |
| 웹           | `web_search`, `web_fetch`                                                                                                                                                                                  |
| 지식 베이스  | `knowledge`, `document`                                                                                                                                                                                    |
| Git          | `git` (commit/push/branch/diff)                                                                                                                                                                            |
| 개발 도구    | `lsp`, `workspace`                                                                                                                                                                                         |
| 작업 관리    | `plan`, `task_system`, `todo_write`, `cron`                                                                                                                                                                |
| 메시징       | `push_notification`, `messaging`                                                                                                                                                                           |
| 데이터베이스 | `database`                                                                                                                                                                                                 |
| 스토리지     | `storage`                                                                                                                                                                                                  |
| 기타         | `agent`, `agent_memory`, `context`, `export`, `integration`, `media`, `media_delivery`, `migration_tool`, `monitor`, `obsidian`, `ocr`, `personality`, `shared_path`, `system_info`, `testing`, `worktree` |

### MCP 프로토콜

`rmcp` 기반의 완전한 MCP (Model Context Protocol) 구현:

- **전송**: stdio 서브프로세스 + Streamable HTTP + WebSocket
- **OAuth 인증**: MCP 서버의 OAuth 인가 흐름 지원
- **도구 디스커버리**: MCP 서버가 노출하는 도구 자동 발견 및 등록
- **MCP 매니저**: 서버 라이프사이클 관리, 헬스 체크, 자동 재연결

### 플러그인 시스템

OpenClaw 호환 3계층 플러그인 아키텍처(내장/번들/외부):

- npm 패키지 설치, 마켓플레이스 UI 검색 및 설치
- 플러그인 매니페스트 정의, 권한 선언, 샌드박스 격리 실행
- 사용자 정의 도구 등록, 에이전트 제공자, Hook 인터셉트
- 스킬 설치기: 플러그인 패키지에서 스킬을 스킬 시스템으로 설치

### 보안

- **AES-256-GCM 암호화**: API 키 및 민감한 설정의 로컬 암호화 저장(`crypto` crate)
- **프롬프트 인젝션 방어**: 4단계 방어 파이프라인(`prompt-guard`) — 패턴 감지 → 구분자 이스케이프 → XML 래퍼 → 신뢰 레이블, 대화/프롬프트 구축/Git/RAG 전 체인에 통합
- **SSRF 방어**: URL 안전성 검사, 내부 네트워크 주소 요청 차단
- **콘텐츠 필터링**: 다중 유형 콘텐츠 안전 필터링
- **속도 제한**: 도구 호출 및 API 요청 토큰 버킷 제한
- **서킷 브레이커**: 연속 실패 시 자동 서킷 브레이크
- **액세스 제어**: 정책 기반 도구 액세스 권한 제어
- **샌드박스 격리**: 에이전트 및 스킬 실행 환경 격리

### 개발자 도구

- **분산 트레이싱** (`telemetry`): OpenTelemetry 통합, Span/Trace 시각화
- **구조화된 로깅**: tracing-subscriber + chrono 타임스탬프
- **리플레이 디버깅**: 에이전트 실행 궤적 기록(`trajectory_recorder`) 및 재생
- **DevTools 패널**: Trace Explorer 타임라인 뷰어, Benchmark Runner, Tool Recommender
- **벤치마크**: Criterion benchmarks(tool_exec / llm_call / search)
- **CI 체크**: `npm run ci:check` 타입 체크, lint, 포맷 검증 통합

### 데스크톱 및 모바일 경험

- **반응형 레이아웃**: CSS 브레이크포인트 기반 데스크톱/태블릿/모바일 적응(3단계: `desktop` / `tablet` / `mobile`)
- **11개 언어**: 간체 중국어, 번체 중국어, 영어, 일본어, 한국어, 프랑스어, 독일어, 스페인어, 러시아어, 힌디어, 아랍어
- **테마 엔진** (`rt-theme`): 다크/라이트 테마 + 다중 프리셋(21th 모노스페이스 테마 포함), Ant Design 6 심층 커스터마이징
- **Monaco 편집기**: 구문 강조, 차이점 미리보기, 다국어 지원
- **xterm.js 터미널**: WebLinks, Unicode 11, 검색
- **가상 스크롤**: @tanstack/react-virtual + react-virtuoso
- **차트 렌더링**: D2 + Mermaid + Recharts
- **Global Copy Menu**: 사용자 정의 텍스트 선택 복사 메뉴, 네이티브 컨텍스트 메뉴 억제
- **Command Palette**: Ctrl+K 글로벌 명령 팔레트
- **시스템 트레이 + 글로벌 단축키 + 자동 시작**: 비침습적 백그라운드 작동
- **자동 업데이트**: 설정 가능한 간격의 GitHub Releases 버전 확인
- **프록시 지원**: HTTP / SOCKS5 프록시 설정
- **클라우드 워크스페이스**: S3 및 WebDAV 스토리지 동기화, 충돌 감지 및 양방향 동기화

### 모바일

- Android APK/AAB(arm64-v8a, armeabi-v7a, x86_64)
- iOS IPA(arm64)
- 모바일 전용 적응: 안전 영역 인셋, 하단 내비게이션, 드로어 내비게이션

---

## 기술 아키텍처

### 기술 스택

| 계층                  | 기술                                     | 버전 |
| --------------------- | ---------------------------------------- | ---- |
| 데스크톱 프레임워크   | Tauri                                    | 2.11 |
| 프론트엔드 프레임워크 | React                                    | 19   |
| 타입 시스템           | TypeScript                               | 7    |
| UI 라이브러리         | Ant Design                               | 6    |
| CSS 프레임워크        | TailwindCSS                              | 4    |
| 상태 관리             | Zustand                                  | 5    |
| 라우팅                | React Router                             | 7    |
| 코드 편집기           | Monaco Editor                            | 0.55 |
| 터미널                | xterm.js                                 | 6    |
| 워크플로우 편집기     | ReactFlow                                | 12   |
| 차트                  | D2 + Mermaid + Recharts                  |      |
| 애니메이션            | Framer Motion                            | 12   |
| 가상 스크롤           | @tanstack/react-virtual + react-virtuoso |      |
| 드래그 앤 드롭        | @dnd-kit                                 | 6    |
| Markdown 렌더링       | markstream-react + stream-markdown       |      |
| i18n                  | i18next + react-i18next                  |      |
| 빌드 도구             | Vite                                     | 8    |
| 테스트                | Vitest + Playwright                      |      |
| 포맷팅                | dprint(TS/JSON/Markdown/TOML) + rustfmt  |      |
| Lint                  | ESLint + Oxlint + Clippy                 |      |

### 백엔드 아키텍처: Harness 의존성 주입

Rust workspace 아키텍처, **32개 crate**, **Harness DI 패턴** 준수:

> 모든 crate는 axagent-harness가 정의한 trait 인터페이스를 통해 분리되며, 런타임에 axagent-runtime이 의존성을 조립하고 주입.
> 의존 방향: `구체적 구현 → harness ← 호출자`

**harness**는 아키텍처의 초석 — 제로 비즈니스 로직, 제로 구체적 구현, trait 정의, 순수 데이터 DTO, 상수, 통합 오류 타입만 포함. 다른 모든 crate에 의해 의존되며, 자체는 어떤 axagent-* crate에도 의존하지 않음(200+ trait 정의, Agent/Provider/Tool/RAG/Storage/MCP/Plugins/Security/Observability/Memory/Learning/Browser/Messaging 등 커버).

```
src-tauri/crates/
├── harness/          # 아키텍처 초석 — trait 인터페이스, DTO, 오류 타입, DI 계약
├── entities/         # SeaORM 엔티티 모델
├── dao/              # 데이터 액세스 계층(CRUD)
├── migration/        # 데이터베이스 마이그레이션
├── crypto/           # AES-256-GCM 암호화/복호화 및 키 관리
├── credential/       # 자격 증명 안전 저장
├── storage/          # 파일 스토리지 추상화(로컬/S3/WebDAV), ZIP 읽기/쓰기
├── cache/            # 인메모리 캐시 계층
├── disk-cache/       # 디스크 파일 캐시
├── search/           # 검색 엔진(FTS5 + sqlite-vec + candle 로컬 임베딩)
├── document-parser/  # 문서 텍스트 추출(PDF/DOCX/XLSX/PPTX)
├── kit/              # 범용 유틸리티(경로/인코딩/해시/날짜)
├── runtime-core/     # 런타임 공통 타입, 설정 상수
├── runtime/          # 런타임 서비스 오케스트레이션 — 모든 30+ crate를 조립하는 DI 컨테이너
├── rt-workflow/      # 워크플로우 엔진 — DAG 오케스트레이션, 노드 실행기, YAML 직렬화
├── rt-messaging/     # 메시징 플랫폼 게이트웨이 — DingTalk/Feishu/QQ/Slack/WeChat/WhatsApp/Telegram/Discord
├── rt-webhook/       # 범용 Webhook 서버
├── rt-dashboard/     # 대시보드 플러그인 프레임워크
├── rt-theme/         # 테마 엔진
├── agent/            # AI 에이전트 코어 — 80+ 모듈
│                     #   ReAct엔진/계층적계획/심층리서치/팩트체크/생각의나무/
│                     #   리플렉션/자체검증/오류복구/RL최적화/LoRA파인튜닝/
│                     #   평가/도구추천/A-B테스트/코디네이터/블랙보드/비전파이프라인/
│                     #   웹검색/학술검색/Wiki컴파일 등
├── orchestrator/     # 에이전트 오케스트레이션 — 멀티 에이전트 스케줄링, DAG 분해, 동적 서브그래프 실행
├── providers/        # 모델 제공자 어댑터
├── tools/            # 도구 시스템 — Tool trait/레지스트리/오케스트레이션/스트리밍/샌드박스/47+내장도구
├── gateway/          # API 게이트웨이 — axum HTTP/WS 서버, OAuth, 속도 제한, Prometheus
├── mcp/              # MCP 프로토콜 — stdio + Streamable HTTP, rmcp 기반
├── trajectory/       # 학습 시스템 — 메모리/스킬 진화/사용자 프로필/드림 통합
├── plugins/          # 플러그인 시스템 — OpenClaw 호환, npm 패키지 설치, 마켓플레이스
├── telemetry/        # 옵저버빌리티 — OpenTelemetry, 구조화된 로깅, 런타임 메트릭
├── prompt-guard/     # 프롬프트 인젝션 방어 — L1-L4 다단계 감지 파이프라인
├── npm/              # npm 레지스트리 클라이언트
└── schema-gen/       # 데이터베이스 스키마 생성 도구
```

### 프론트엔드 아키텍처

```
src/
├── pages/            # 페이지(서브 페이지 포함 23+)
│   ├── ChatPage           # 채팅 인터페이스 — 사이드바/메시지 스트림/Agent 패널/멀티 탭
│   ├── DashboardPage      # 대시보드 — 사용 통계/모델 분포/트렌드 차트
│   ├── WorkflowPage       # 워크플로우 편집기 — ReactFlow DAG 시각화
│   ├── KnowledgeHubPage   # 지식 베이스 관리 — 문서 업로드/인덱싱/검색
│   ├── MemoryPage         # 메모리 관리
│   ├── SkillsPage         # 스킬 마켓플레이스
│   ├── SettingsPage       # 설정 패널 — 40+ 설정 항목
│   ├── TerminalPage       # 내장 터미널 — xterm.js
│   ├── FilesPage          # 파일 관리
│   ├── GatewayLinkPage    # API 게이트웨이 및 외부 링크 관리
│   ├── QuickBarPage       # 퀵바(독립 창)
│   ├── DynamicUIManagerPage / DynamicPageViewer  # 동적 UI 엔진
│   ├── WikiGraphPage / WikiEditPage / IngestPage # LLM Wiki
│   ├── LearningGraphPage  # 학습 그래프
│   ├── FineTunePage       # LoRA 파인튜닝
│   ├── PersonaPage        # 페르소나 관리
│   ├── WorkflowMarketplace # 워크플로우 마켓플레이스
│   ├── DevTools/          # TraceExplorer / BenchmarkRunner / ToolRecommender
│   └── Workflow/          # WorkflowListPage
│
├── components/       # 28 모듈, 450+ 컴포넌트
│   ├── chat/         # 채팅(메시지 스트림/입력/ChatView/TabBar/RightPanel/첨부파일/도구 호출 렌더링)
│   ├── layout/       # 레이아웃 — 17 컴포넌트
│   │                 #   AppInitializer / Sidebar / ContentArea / TitleBar /
│   │                 #   CommandPalette / GlobalCopyMenu / GlobalErrorBoundary /
│   │                 #   GlobalStatusBar / ErrorNotificationToast / AppHeader /
│   │                 #   BackendStatusIndicator / IpcReconnectBanner /
│   │                 #   ModuleErrorBoundary / NotificationBell / UserProfileModal 등
│   ├── agent/        # Agent 패널/엔트리/미니 패널
│   ├── workflow/     # 워크플로우 편집기(노드/엣지/패널/템플릿/AI 보조)
│   ├── settings/     # 설정 패널(40+ 서브 컴포넌트)
│   ├── skill/        # 스킬 편집기/렌더러/플로팅 패널
│   ├── dynamicUI/    # 동적 UI 컴포넌트 레지스트리(26 내장 컴포넌트)
│   ├── gateway/      # API 게이트웨이 관리
│   ├── files/        # 파일 관리
│   ├── terminal/     # 터미널 컴포넌트
│   ├── search/       # 검색 인터페이스
│   ├── benchmark/    # 벤치마크 패널
│   ├── decomposition/# 스킬 분해 및 도구 생성
│   ├── devtools/     # Trace/Span 타임라인 + RL Training 패널
│   ├── approval/     # 승인 워크플로우 UI
│   ├── recommendation/ # 도구/모델 추천
│   ├── onboarding/   # WelcomeWizard / InteractiveTutorial
│   ├── help/         # 도움말 패널
│   ├── notification/ # 알림 컴포넌트
│   ├── proactive/    # 능동적 제안
│   ├── llm-wiki/     # LLM Wiki 컴포넌트
│   ├── wiki/         # Wiki 컴포넌트
│   ├── fine-tune/    # 파인튜닝 UI
│   ├── trace/        # Trace 컴포넌트
│   ├── style/        # 스타일/테마
│   ├── shared/       # 공유 컴포넌트(ErrorBoundary / PageContextProvider)
│   └── common/       # 공통 컴포넌트(Icon 등)
│
├── stores/           # Zustand 상태 관리
│   ├── domain/       # 10 핵심 비즈니스 스토어(대화/스트림/압축/설정/멀티모델 등)
│   ├── feature/      # 48 기능 모듈 스토어(에이전트/워크플로우/지식/스킬/게이트웨이/메모리/터미널 등)
│   └── devtools/     # 4 개발자 도구 스토어
│
├── hooks/            # React Hooks(단축키/명령팔레트/반응형/스크롤바/테마/아바타 등)
├── lib/              # 유틸리티 라이브러리(invoke/pageRegistry/shortcuts/skillLifecycle/
│                     #   chartGenerator/codeExecutor/tokenEstimator/workflowLayout 등 45+ 모듈)
├── types/            # TypeScript 타입 정의
├── theme/            # Shadcn 테마 엔진
├── i18n/             # 11개 언어 번역 파일(zh-CN/zh-TW/en-US/ja/ko/fr/de/es/ru/hi/ar)
├── constants/        # 상수 및 기능 플래그
└── sdk/              # 외부 통합 SDK
```

### 기능 플래그

프로젝트는 `featureFlags.ts`로 점진적 기능 롤아웃 관리:

| 플래그              | 상태 | 설명                                      |
| ------------------- | ---- | ----------------------------------------- |
| `AGENT_IN_THE_LOOP` | ✅   | 글로벌 Agent Panel + 페이지 컨텍스트 주입 |
| `DYNAMIC_UI`        | ✅   | 동적 UI 빌더 엔진                         |
| `SELF_EVOLUTION_UI` | ❌   | 자가 진화 프론트엔드 제어 패널            |
| `NL_EXTENSION`      | ❌   | 자연어 기반 동적 비즈니스 확장            |

### Tauri 플러그인

| 플러그인            | 용도                            |
| ------------------- | ------------------------------- |
| `autostart`         | 부팅 시 자동 시작               |
| `clipboard-manager` | 클립보드 읽기/쓰기              |
| `dialog`            | 파일 선택 대화상자              |
| `fs`                | 파일 시스템 액세스              |
| `global-shortcut`   | 글로벌 단축키 등록              |
| `notification`      | 시스템 알림                     |
| `opener`            | 외부 링크/파일 열기             |
| `process`           | 프로세스 관리                   |
| `updater`           | 자동 업데이트                   |
| `mcp-bridge`        | MCP 프로토콜 브릿지(비 Android) |

---

## 데이터 디렉터리

```
~/.axagent/                    # 애플리케이션 설정
├── axagent.db                 # SQLite 메인 데이터베이스 (SeaORM)
├── master.key                 # AES-256 마스터 키
├── vector_db/                 # sqlite-vec 벡터 인덱스
└── ssl/                       # 자체 서명 SSL 인증서

~/Documents/axagent/          # 사용자 파일
├── images/                   # 이미지 첨부
├── files/                    # 파일 첨부
└── backups/                  # 자동 백업
```

---

## 빠른 시작

### 사전 요구사항

- [Node.js](https://nodejs.org/) 20+
- [Rust](https://www.rust-lang.org/) 1.75+ (edition 2024)
- [npm](https://www.npmjs.com/) 10+
- Windows: [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) (MSVC + Windows SDK)
- macOS: Xcode Command Line Tools
- Linux: `build-essential` + `libwebkit2gtk-4.1-dev` + `libssl-dev`

### 개발

```bash
git clone https://github.com/polite0803/AxAgent.git
cd AxAgent
npm install
npm run tauri dev      # 개발 모드 (Vite HMR + Tauri 창)
```

### 빌드

```bash
npm run tauri build    # 데스크톱 프로덕션 빌드

npm run tauri:android:build   # Android 빌드
npm run tauri:ios:build       # iOS 빌드
```

데스크톱 빌드 결과물은 `src-tauri/target/release/`에 있습니다.

### 테스트

```bash
npm run test           # 프론트엔드 단위 테스트 (Vitest watch)
npm run test:run       # 프론트엔드 단위 테스트 (단일 실행)
npm run test:e2e       # E2E 테스트 (Playwright)

# Rust 백엔드 테스트
cd src-tauri && cargo nextest run
cd src-tauri && cargo test

# 타입 체크 & Lint
npm run typecheck
cd src-tauri && cargo clippy -- -D warnings
npm run format         # dprint 포맷팅
npm run lint:eslint    # ESLint 체크
npm run contracts      # API 계약 체크

# 전체 CI 체크
npm run ci:check
```

### 스크립트

| 명령                     | 용도                   |
| ------------------------ | ---------------------- |
| `npm run bump`           | 대화형 버전 업그레이드 |
| `npm run docs`           | TypeDoc 문서 생성      |
| `npm run skill:create`   | 새 스킬 스캐폴드 생성  |
| `npm run skill:validate` | 스킬 정의 검증         |
| `npm run check:types`    | 타입 일관성 체크       |

---

## 플랫폼 지원

| 플랫폼  | 아키텍처                              |
| ------- | ------------------------------------- |
| Windows | x86_64, ARM64                         |
| macOS   | Apple Silicon (arm64), Intel (x86_64) |
| Linux   | x86_64, ARM64                         |
| Android | arm64-v8a, armeabi-v7a, x86_64        |
| iOS     | arm64                                 |

---

## 라이선스

본 프로젝트는 [AGPL-3.0-only](LICENSE) 라이선스로 오픈소스 공개됩니다.

---

## 감사의 말

AxAgent는 많은 뛰어난 오픈소스 프로젝트 위에 구축되었습니다:

- [Tauri](https://tauri.app/) — 크로스플랫폼 데스크톱 프레임워크
- [React](https://react.dev/) + [Ant Design](https://ant.design/) — 프론트엔드 UI
- [SeaORM](https://www.sea-ql.org/SeaORM/) — Rust ORM
- [sqlite-vec](https://github.com/asg017/sqlite-vec) — 벡터 검색
- [candle](https://github.com/huggingface/candle) — 로컬 임베딩 추론
- [rmcp](https://github.com/nicholasxjy/rmcp) — Rust MCP SDK
- [ReactFlow](https://reactflow.dev/) — 비주얼 워크플로우 편집기
- [axum](https://github.com/tokio-rs/axum) — HTTP 프레임워크
- [Monaco Editor](https://microsoft.github.io/monaco-editor/) — 코드 편집기
- [xterm.js](https://xtermjs.org/) — 터미널 에뮬레이터
- [Zustand](https://zustand.docs.pmnd.rs/) — 상태 관리
- [Framer Motion](https://www.framer.com/motion/) — 애니메이션 라이브러리
- [Recharts](https://recharts.org/) — 차트 라이브러리
