# AxAgent

[![Release](https://img.shields.io/github/v/release/polite0803/AxAgent?style=flat-square)](https://github.com/polite0803/AxAgent/releases)
[![Build](https://img.shields.io/github/actions/workflow/status/polite0803/AxAgent/release.yml?style=flat-square)](https://github.com/polite0803/AxAgent/actions)
![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux%20%7C%20Android%20%7C%20iOS-blue?style=flat-square)
![License](https://img.shields.io/badge/license-AGPL--3.0-green?style=flat-square)

<p align="center">
  <a href="./media/poster-axagent.svg">
    <img src="./media/poster-axagent.svg" alt="AxAgent Poster" width="80%" />
  </a>
</p>

**AxAgent**는 Tauri 2 기반의 크로스 플랫폼 AI 데스크톱 클라이언트(Windows / macOS / Linux / Android / iOS)로, AI 기반의 일상적인 개발, 연구, 지식 관리 및 자동화 워크벤치를 지향합니다. ReAct 에이전트 엔진, 인지 라우팅(3단계 계층 라우팅 + 검색 증강 라우팅 RAR), 시각적 워크플로우 오케스트레이션, 로컬 RAG 지식 베이스, MCP 프로토콜 확장, 멀티모델 통합 게이트웨이, 브라우저 자동화 및 컴퓨터 제어 등의 기능을 내장하여 AI를 "대화"에서 "실행"으로 이끕니다.

> **언어 버전**: [简体中文](./README.md) | [English](./README-EN.md) | [繁體中文](./README-ZH-TW.md) | [日本語](./README-JA.md) | [한국어](./README-KO.md) | [Français](./README-FR.md) | [Deutsch](./README-DE.md) | [Español](./README-ES.md) | [Русский](./README-RU.md) | [हिन्दी](./README-HI.md) | [العربية](./README-AR.md)

---

## 프로젝트 포지셔닝

AxAgent는 세 가지 핵심 문제를 해결합니다:

1. **멀티모델 통합 연결 및 지능형 스케줄링** — 단일 인터페이스에서 OpenAI, Anthropic Claude, Google Gemini, DeepSeek, Qwen, GLM, Kimi, Wenxin(文心), Ollama 로컬 모델 및 모든 OpenAI 호환 API를 동시에 사용할 수 있으며, 다중 Key 할당량 자동 로테이션, 작업 유형별 지능형 라우팅, 스트리밍 비교를 지원합니다.
2. **대화에서 실행까지의 AI 클로즈드 루프** — 163+ 내장 도구 + 시각적 워크플로우 + MCP 확장 + 브라우저/컴퓨터 제어로, AI가 파일을 조작하고, 코드를 실행하고, Git을 관리하고, 작업을 스케줄링할 수 있습니다.
3. **로컬 우선 데이터 주권** — 대화 기록, 지식 베이스, 메모리, 설정이 모두 로컬 SQLite 데이터베이스에 저장되며, API Key는 AES-256-GCM으로 암호화됩니다. 타사 클라우드 서비스 없이도 핵심 기능을 실행할 수 있습니다.

---

## 핵심 기능

### 인지 라우팅 시스템(Cognitive Router)

AxAgent는 `cognitive_query`를 모든 대화의 통합 진입점으로 사용하며, **3단계 계층 라우팅**을 통해 사용자 의도를 구체적인 기능에 매핑합니다:

- **L1 도메인 라우팅** (`domain_router`): 규칙 + LLM 폴백으로 9대 비즈니스 도메인(데이터 분석 / 콘텐츠 제작 / 커뮤니케이션 / 운영 / AI 미디어 / 금융 / 자동화 / 일반 등)을 식별합니다.
- **L2 클러스터 라우팅** (`cluster_router`): 도메인 내에서 기능 클러스터를 식별합니다(27개 클러스터, 8대 비즈니스 도메인 커버).
- **L3 기능 라우팅**: **검색 증강 라우팅(RAR)** — 기능 벡터 라이브러리에서 Top-K 유사 워크플로우를 리콜하여 Prompt에 주입하고, 워크플로우 DAG 그래프 경로 탐색과 결합하여 경로 주소(예: `/finance/stock_analysis/tech`)와 실행 모드를 출력합니다.
- **실행 모드**: `Ask / Plan / Act / Workflow / Direct / Delegate / ParameterExtract / Clarify`를 신뢰도에 따라 자동 선택합니다.
- **기능 시스템**: 통합 레지스트리(`CapabilityRegistry`) + 벡터 인덱스(`CapabilityIndexer`) + 하이브리드 검색(`CapabilityRetriever`, 벡터 + BM25 + 태그 하드 매칭 + 네거티브 샘플 제외)
- **시스템 기능 격리**: 인지 오케스트레이터와 비즈니스 워크플로우를 물리적으로 격리하며, 시스템 기능에는 `SYSTEM_ONLY` 가시성 마커가 부여되고, 라우팅 계층에 자체 참조 서킷 브레이커가 내장되어 자기 지시 패러독스를 방지합니다.
- **3단계 라우팅을 워크플로우 DAG로 구현**: 4개의 사전 설정 라우팅 워크플로우 템플릿(메인 오케스트레이션 ~20 노드 + L1/L2/L3 서브 라우팅)을 `rt-workflow` 엔진이 실행합니다.

### 멀티모델 엔진

- **13가지 공급자 어댑터**: OpenAI(Chat Completions + Responses + Realtime), Anthropic Claude, Google Gemini, DeepSeek, Qwen, GLM, Kimi, Wenxin Yiyan(文心一言), Ollama, Llama.cpp(GGUF 로컬 모델), OpenClaw, Hermes 및 모든 OpenAI 호환 API
- **다중 Key 로테이션**: 동일 공급자의 여러 API Key를 할당량에 따라 자동 로테이션하고, 단일 Key 제한 시 자동 전환합니다.
- **지능형 라우팅**: 작업 유형(코드 리뷰 / 요약 / 번역 / 일반)에 따라 최적의 모델을 자동 선택하며, 사용자 정의 규칙을 지원합니다.
- **공급자 상태 모니터링**: 성공률, 지연 시간, 가용 상태를 실시간 추적하고, 단계별 자동 다운그레이드를 지원합니다.
- **AI 이미지 생성**: DALL-E 3 및 Flux 다중 크기 프리셋
- **실시간 음성**: OpenAI Realtime API 기반 WebSocket 음성 대화로, 인터럽트 및 스트리밍 전사를 지원합니다.

### 에이전트 시스템(ReAct 엔진)

- **계층형 플래너** (`hierarchical_planner`): 복잡한 작업을 Phase → Task 구조화 계획으로 분해하고 DAG 토폴로지로 컴파일하여 실행합니다.
- **딥 리서치** (`deep_research`): 검색 계획, 검색 실행, 콘텐츠 종합, 인용 추적을 포함한 다중 소스 검색 오케스트레이션
- **사실 확인** (`fact_checker`): AI 기반 사실 검증으로, 출처 분류기와 신뢰도 평가를 포함합니다.
- **사고의 나무** (`tree_of_thoughts`): 다중 경로 추론 탐색, 분기 평가 및 백트래킹
- **리플렉터** (`reflector`): 작업 실행 후 자기 평가 및 개선 제안
- **자기 검증** (`self_verifier`): 추론 결과 자동 검증으로, 루프 감지를 포함합니다.
- **오류 복구** (`error_recovery_engine`): 오류 유형 분류 → 복구 전략 선택 → 자동 재시도 또는 계획 조정, 지수 백오프를 지원합니다.
- **A/B 테스트** (`ab_testing`): 서로 다른 추론 전략의 비교 평가
- **평가 시스템** (`evaluator`): 내장 벤치마크 테스트 프레임워크
- **LoRA 파인튜닝** (`fine_tune`): 내장 학습 파이프라인으로 LoRA 어댑터 관리를 지원합니다.
- **RL 옵티마이저** (`rl_optimizer`): 경험 피드백 기반 정책 강화 학습

**멀티 에이전트 협업**:

- 마스터-슬레이브 조정 아키텍처로, 하위 에이전트를 병렬 실행하고 의존성 인지 스케줄링을 수행합니다.
- 에이전트 간 정보 교환을 위한 공유 블랙보드
- 적대적 토론 모드(Pro/Con 라운드와 논점 강도 점수)
- Swarm 클러스터 모드, 다중 프로세스 에이전트 클러스터
- 능동 모드: 에이전트가 능동적으로 제안과 작업을 시작할 수 있습니다.

**컴퓨터 제어**: AI 기반 마우스 클릭, 키보드 입력, 화면 스크롤, 3단계 권한(기본 / 편집 허용 / 전체 액세스), 샌드박스 경로 격리

**브라우저 자동화**: CDP 프로토콜을 통해 브라우저를 제어하며, 탐색, 스크린샷, 클릭, 폼 작성, 텍스트 추출을 지원합니다.

### 스킬 시스템

- **스킬 마켓**: 커뮤니티 스킬 탐색 및 설치
- **AI 보조 생성**: 자연어 제안에서 스킬 구조를 자동 생성합니다(`skill:create`).
- **스킬 진화** (`evolution_engine`): 실행 피드백을 기반으로 스킬을 자동 분석하고 개선합니다.
- **의미 매칭**: 대화 컨텍스트의 의미를 기반으로 관련 스킬을 자동 추천합니다.
- **스킬 분해** (`skill_decomposition`): 복잡한 작업을 원자적 스킬 조합으로 자동 분해합니다.
- **도구 생성**: AI가 새 도구를 생성하고 등록합니다.
- **샌드박스 실행**: 스킬이 격리된 샌드박스에서 안전하게 실행됩니다.

### 시각적 워크플로우

ReactFlow 12 기반의 드래그 앤 드롭 DAG 워크플로우 편집기:

- **32가지 노드 유형**: 트리거, 에이전트, LLM 호출, 조건 분기, 병렬 분기, 루프, 병합, 지연, 도구 호출, 코드 실행, 하위 워크플로우, 벡터 검색, 문서 파싱, 검증, 종료, HTTP 요청, Switch, 데이터베이스 쿼리, 알림, 승인, 파일 작업, 데이터 변환, Webhook 전송, 로그, LLM 분류기, 집계기, 이메일, 토론, Swarm, 멀티 에이전트, 스토리지, 비즈니스 규칙
- **Kahn 위상 정렬 실행**: 순환 의존성을 자동 감지하고 병렬 파이프라인을 스케줄링합니다.
- **내장 템플릿**: 코드 리뷰, 버그 수정, 문서 생성, 테스트, 리팩토링, 탐색, 성능 분석, 보안 검토, 기능 개발
- **YAML 직렬화**: 워크플로우 정의 가져오기/내보내기
- **버전 관리**: 템플릿 버전 제어
- **AI 보조 설계**: AI 보조 워크플로우 설계, 노드 추천 및 진단

### 지식 관리

- **다중 지식 베이스 RAG**: 문서 업로드 → 자동 파싱(PDF/DOCX/XLSX/PPTX/TXT) → 청크 분할 → 벡터 인덱싱
- **하이브리드 검색**: 벡터 유사도(sqlite-vec + candle 로컬 임베딩) + BM25 전문 검색(FTS5), 혼합 정렬
- **Self-RAG**: 검색 결과 자동 반성 및 검증
- **재정렬**: Cross-encoder 결과 재정렬
- **지식 그래프**: 엔티티 추출 → 관계 구축 → 시각화 그래프
- **파일 모니터링**: `notify` 기반 실시간 파일 변경 감지, 자동 증분 인덱싱
- **LLM Wiki**: AI 보조 Wiki 컴파일러 및 검증기

### 메모리 시스템

- **다중 네임스페이스 메모리**: 프로젝트/주제별 격리, 수동 입력과 AI 자동 추출 지원
- **지속성 통합**: Honcho 및 Mem0 클로즈드 루프 메모리
- **사용자 프로필**: 코드 스타일, 기술 스택 선호도, 커뮤니케이션 스타일 자동 학습
- **스타일 전이**: 코드 스타일 특징 추출 → AI 생성 코드에 적용
- **드림 통합**: 백그라운드에서 메모리 조각과 행동 패턴을 자동 통합하여 구조화된 지식 생성
- **프로젝트 메모리**: 프로젝트 차원의 컨텍스트 지속화

### API 게이트웨이

`axum` 기반의 HTTP + WebSocket 게이트웨이 내장:

- **호환 엔드포인트**: OpenAI `/v1/chat/completions`, Claude Messages API, Gemini API 및 OpenAI Responses와 Realtime WebSocket
- **Key 관리**: 액세스 키 생성, 폐기, 활성화/비활성화, 만료 시간 지원
- **사용량 추적**: Key/공급자/날짜별 요청 수와 token 소비 통계, Prometheus 메트릭 내보내기
- **속도 제한**: `governor` 기반 토큰 버킷 알고리즘
- **SSL/TLS**: 내장 자체 서명 인증서(`rcgen`), 사용자 정의 인증서 지원
- **외부 연결**: Claude CLI, OpenCode 등 외부 도구를 원클릭으로 통합하고 API Key를 자동 동기화합니다.
- **실시간 티켓**: WebSocket 연결의 안전한 전달을 위한 HMAC 기반 임시 인증 티켓
- **Server 모드**: 선택적 `axagent-server` 바이너리로, 데스크톱 앱의 기능을 서비스 형태로 외부에 제공합니다.

### 메시지 플랫폼 통합

`rt-messaging`을 통해 다중 플랫폼 게이트웨이를 구현하며, **DingTalk(钉钉), Feishu(飞书), QQ, Slack, WeChat(微信), WhatsApp, Telegram, Discord**의 메시지 수신, 명령 파싱 및 AI 자동 응답을 지원합니다.

### 도구 시스템

**163+ 내장 도구**가 `Tool` trait을 통해 통합 등록되며, 15대 카테고리를 커버합니다:

| 카테고리      | 도구 예시                                                                                                                                                               |
| ------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 파일 작업     | `file_read`, `file_write`, `file_edit`, `glob`, `grep`, 디렉터리/삭제/이동 등 11개                                                                                      |
| Shell/Web     | `bash`, `web_fetch`, `web_search`                                                                                                                                       |
| 네트워크      | `http_request`, `ping`, `dns_lookup`, `json_api`, `rss_reader`, `graphql`, `websocket`                                                                                  |
| 브라우저      | `browser_navigate`, `browser_click`, `browser_fill`, `browser_screenshot` 등 10개(CDP)                                                                                  |
| 컴퓨터 제어   | `computer_use`(마우스/키보드/스크린샷)                                                                                                                                  |
| Git           | `git_status`, `git_diff`, `git_commit`, `git_log`, `git_branch`, `git_review`                                                                                           |
| 지식 베이스   | `list_knowledge_bases`, `search_knowledge`, `add_knowledge_document` 등 6개                                                                                             |
| 작업 관리     | `todo_write`, `task_*`(6개), `cron_*`(3개), `plan` 관련                                                                                                                 |
| 메시지 푸시   | `push_notification`, `send_message`, 팀 협업 도구                                                                                                                       |
| 데이터베이스  | `database_query`, `database_list_tables`, `database_migration_status`                                                                                                   |
| 스토리지      | `get_storage_info`, `upload_storage_file`, `download_storage_file` 등 5개                                                                                               |
| 내보내기/포맷 | `export_word`, `export_pdf`, `export_xlsx`, `export_pptx`, `render_markdown` 등 9개                                                                                     |
| OCR           | `ocr_image`, `ocr_detect_langs`                                                                                                                                         |
| Obsidian      | `obsidian_search`, `obsidian_read`, `obsidian_backlinks` 등 9개                                                                                                         |
| 기타          | `agent`, `delegate_task`, `skills_*`, `lsp`, `repl`, `monitor`, `workspace_*`, `session_search`, `generate_image`, `sequential_thinking`, CI/CD, DevOps, RPC, 테스트 등 |

### MCP 프로토콜

`rmcp` 기반의 완전한 MCP(Model Context Protocol) 구현:

- **전송 계층**: stdio 서브프로세스 + Streamable HTTP + SSE
- **OAuth 인증**: MCP 서버의 OAuth 인증 흐름 지원
- **도구 발견**: MCP 서버가 노출하는 도구를 자동 발견하고 등록합니다.
- **MCP 매니저**: 서버 수명 주기 관리, 상태 확인, 자동 재연결

### 플러그인 시스템

OpenClaw 호환 3단계 플러그인 아키텍처(내장/번들/외부):

- npm 패키지 설치, 내장 마켓 UI 검색 및 설치
- 플러그인 manifest 정의, 권한 선언, 샌드박스 격리 실행
- 사용자 정의 도구 등록, Agent 제공자, Hook 인터셉트
- 스킬 설치기: 플러그인 패키지에서 스킬 시스템으로 스킬 설치

### 동적 UI 엔진

- **Schema 기반**: JSON Schema를 통해 코드 작성 없이 선언적으로 인터페이스를 구축합니다.
- **31개 내장 컴포넌트**: 컨테이너(7) / 데이터 표시(6) / 폼(9) / 미디어(4) / 기타(5)
- **데이터 바인딩**: 선언적 데이터 소스 바인딩 및 조건부 렌더링
- **NL2UI**: 자연어로 동적 UI 인터페이스를 직접 생성

### ACP 클라이언트 SDK

- **ACP(Agent Client Protocol)**: 이중 언어 SDK(TypeScript + Python), 제로 서드파티 의존성
- 세션 관리, Prompt 전송, 도구 호출 기록, WebSocket 이벤트 스트림
- `/acp/v1/*` 엔드포인트를 통해 AxAgent 서비스와 통신합니다.

### 보안

- **AES-256-GCM 암호화**: API Key 및 민감한 설정을 로컬에 암호화하여 저장합니다(`crypto` crate).
- **프롬프트 인젝션 방어**: 4단계 방어 파이프라인(`prompt-guard`) — 패턴 감지 → 구분자 이스케이프 → XML 래퍼 → 신뢰 태그, 세션, 프롬프트 구축, Git, RAG 전 체인에 통합됩니다.
- **SSRF 방어**: URL 보안 검사로 내부 네트워크 주소에 대한 요청을 차단합니다.
- **콘텐츠 필터링**: 다중 유형 콘텐츠 안전 필터링
- **속도 제한**: 도구 호출 및 API 요청 토큰 버킷 제한
- **서킷 브레이커**: 연속 실패 시 자동 차단
- **접근 제어**: 정책 기반 도구 접근 권한 제어
- **샌드박스 격리**: 에이전트 및 스킬 실행 환경 격리

### 개발자 도구

- **분산 추적** (`telemetry`): OpenTelemetry 통합, Span/Trace 시각화
- **구조화 로그**: tracing-subscriber + chrono 타임스탬프
- **재생 디버깅**: 에이전트 실행 궤적 녹화(`trajectory_recorder`) 및 재생
- **DevTools 패널**: Trace Explorer 타임라인 뷰어, Benchmark Runner, Tool Recommender
- **벤치마크**: Criterion 벤치마크(tool_exec / llm_call / search)
- **CI 검사**: `npm run ci:check`가 타입 검사, lint, 포맷 검증을 통합합니다.

### 데스크톱 및 모바일 경험

- **반응형 레이아웃**: CSS 브레이크포인트로 데스크톱/태블릿/모바일에 자동 대응(3단계 디바이스 레이아웃: `desktop` / `tablet` / `mobile`)
- **11개 언어**: 중국어 간체, 중국어 번체, 영어, 일본어, 한국어, 프랑스어, 독일어, 스페인어, 러시아어, 힌디어, 아랍어
- **테마 엔진** (`rt-theme`): 다크/라이트 테마 + 여러 프리셋, Ant Design 6 심층 커스터마이징
- **Monaco 편집기**: 구문 하이라이팅, 차이 미리보기, 다국어 지원
- **xterm.js 터미널**: WebLinks, Unicode 11, 검색
- **가상 스크롤**: @tanstack/react-virtual + react-virtuoso
- **차트 렌더링**: D2 + Mermaid + Recharts + Sigma(그래프)
- **Command Palette**: Ctrl+K 전역 명령 팔레트
- **시스템 트레이 + 전역 단축키 + 자동 시작**: 방해 없는 백그라운드 실행
- **자동 업데이트**: 구성 가능한 간격의 GitHub Releases 버전 감지
- **프록시 지원**: HTTP / SOCKS5 프록시 설정
- **클라우드 워크스페이스**: S3 및 WebDAV 스토리지 동기화, 충돌 감지 및 양방향 동기화

### 모바일

- Android APK/AAB(arm64-v8a, armeabi-v7a, x86_64)
- iOS IPA(arm64)
- 모바일 전용 적응: 안전 영역 적응, 하단 내비게이션, Drawer 내비게이션

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
| 차트                  | D2 + Mermaid + Recharts + Sigma          |      |
| 애니메이션            | Framer Motion                            | 12   |
| 가상 스크롤           | @tanstack/react-virtual + react-virtuoso |      |
| 드래그 앤 드롭        | @dnd-kit                                 | 6    |
| Markdown 렌더링       | markstream-react + stream-markdown       |      |
| 국제화                | i18next + react-i18next                  |      |
| 빌드 도구             | Vite                                     | 8    |
| 테스트                | Vitest + Playwright                      |      |
| 포맷팅                | dprint(TS/JSON/Markdown/TOML) + rustfmt  |      |
| Lint                  | ESLint + Oxlint + Clippy                 |      |

### 백엔드 아키텍처: Harness 의존성 주입 패턴

Rust workspace 아키텍처를 채택하며 **37개 멤버**(메인 crate + 35개 라이브러리 crate + schema-gen)로 구성되고, **Harness 의존성 주입 아키텍처**를 따릅니다:

> 모든 crate는 axagent-harness가 정의한 trait 인터페이스로 디커플링되며, 런타임에서 axagent-runtime이 의존성을 조립하고 주입합니다.
> 의존성 방향: `구체적 구현 → harness ← 호출자`

**harness**는 아키텍처의 초석입니다 — 비즈니스 로직이 없고 구체적 구현이 없으며, trait 정의, 순수 데이터 DTO, 상수 및 통합 오류 유형만 포함합니다. 다른 모든 crate가 의존하며, 자체는 어떤 axagent-* crate에도 의존하지 않습니다(200+ trait 정의로 Agent/Provider/Tool/RAG/스토리지/MCP/플러그인/보안/관측성/메모리/학습/브라우저/메시지/인지 라우팅 등을 커버).

```
src-tauri/crates/
├── harness/          # 아키텍처 초석 — trait 인터페이스, DTO, 오류 유형, DI 계약
├── entities/         # SeaORM 엔티티 모델
├── dao/              # 데이터 접근 계층(CRUD)
├── migration/        # 데이터베이스 마이그레이션
├── crypto/           # AES-256-GCM 암복호화 및 키 관리
├── credential/       # 자격 증명 안전 저장
├── storage/          # 파일 스토리지 추상화(로컬/S3/WebDAV), ZIP 읽기/쓰기
├── cache/            # 메모리 캐시 계층
├── disk-cache/       # 디스크 파일 레벨 캐시
├── search/           # 검색 엔진(FTS5 + sqlite-vec + candle 로컬 임베딩)
├── document-parser/  # 문서 텍스트 추출(PDF/DOCX/XLSX/PPTX)
├── kit/              # 공용 도구 모음(경로/인코딩/해시/날짜)
├── runtime-core/     # 런타임 공용 타입, 설정 상수
├── runtime/          # 런타임 서비스 오케스트레이션 — 전체 crate의 DI 컨테이너 조립
├── rt-workflow/      # 워크플로우 엔진 — DAG 오케스트레이션, 노드 실행기, YAML 직렬화
├── rt-messaging/     # 메시지 플랫폼 게이트웨이 — DingTalk/Feishu/QQ/Slack/WeChat/WhatsApp/Telegram/Discord
├── rt-webhook/       # 공용 Webhook 서버
├── rt-dashboard/     # 대시보드 플러그인 프레임워크
├── rt-theme/         # 테마 엔진
├── agent/            # AI 에이전트 코어 — 80+ 모듈
│                     #   ReAct엔진/계층형 계획/딥 리서치/사실 확인/사고의 나무/반성/
│                     #   자기 검증/오류 복구/RL 최적화/LoRA 파인튜닝/평가/도구 추천/A/B 테스트/
│                     #   코디네이터/블랙보드/비전 파이프라인/웹 검색/학술 검색/Wiki 컴파일 등
├── orchestrator/     # 에이전트 오케스트레이션 — 멀티 에이전트 스케줄링, DAG 분해, 동적 서브그래프 실행
├── providers/        # 모델 공급자 어댑터(13종)
├── tools/            # 도구 시스템 — Tool trait/레지스트리/오케스트레이션/스트리밍/샌드박스/163+ 내장 도구
├── gateway/          # API 게이트웨이 — axum HTTP/WS 서버, OAuth, 속도 제한, Prometheus
├── mcp/              # MCP 프로토콜 — stdio + Streamable HTTP + SSE, rmcp 기반
├── trajectory/       # 학습 시스템 — 메모리/스킬 진화/사용자 프로필/드림 통합
├── plugins/          # 플러그인 시스템 — OpenClaw 호환, npm 패키지 설치, 마켓
├── telemetry/        # 관측성 — OpenTelemetry, 구조화 로그, 런타임 메트릭
├── prompt-guard/     # 프롬프트 인젝션 방어 — L1-L4 다단계 감지 파이프라인
├── npm/              # npm 레지스트리 클라이언트
├── crdt/             # 협업 편집 데이터 구조
├── device/           # 디바이스 관리
├── axagent-mobile/   # 모바일 적응 계층
├── agent-macro/      # 에이전트 매크로
├── agent-command-types/ # 에이전트 명령 타입
└── schema-gen/       # 데이터베이스 Schema 생성 도구
```

### 프론트엔드 아키텍처

```
src/
├── pages/            # 페이지(24개)
│   ├── ChatPage           # 대화 메인 인터페이스 — 사이드바/메시지 스트림/Agent 패널/멀티 Tab
│   ├── DashboardPage      # 데이터 대시보드 — 사용량 통계/모델 분포/추세 차트
│   ├── WorkflowPage       # 워크플로우 편집기 — ReactFlow DAG 시각화
│   ├── KnowledgeHubPage   # 지식 베이스 관리 — 문서 업로드/인덱싱/검색
│   ├── MemoryPage         # 메모리 관리
│   ├── SkillsPage         # 스킬 마켓
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
│   ├── WorkflowMarketplace # 워크플로우 마켓
│   ├── DevTools/          # TraceExplorer / BenchmarkRunner / ToolRecommender
│   └── Workflow/          # WorkflowListPage
│
├── components/       # 33개 모듈, 500+ 컴포넌트
│   ├── chat/         # 대화(메시지 스트림/입력/ChatView/TabBar/RightPanel/첨부/도구 호출 렌더링)
│   ├── layout/       # 레이아웃 — AppInitializer / Sidebar / ContentArea / TitleBar /
│   │                 #   CommandPalette / GlobalCopyMenu / GlobalErrorBoundary /
│   │                 #   GlobalStatusBar / ErrorNotificationToast / AppHeader 등
│   ├── agent/        # Agent 패널/진입점/미니 패널
│   ├── workflow/     # 워크플로우 편집기(노드/엣지/패널/템플릿/AI 보조)
│   ├── settings/     # 설정 패널(40+ 하위 컴포넌트)
│   ├── skill/        # 스킬 편집기/렌더러/플로팅 패널
│   ├── dynamicUI/    # 동적 UI 컴포넌트(31개 내장 컴포넌트)
│   ├── gateway/      # API 게이트웨이 관리
│   ├── files/        # 파일 관리
│   ├── terminal/     # 터미널 컴포넌트
│   ├── search/       # 검색 인터페이스
│   ├── benchmark/    # 벤치마크 패널
│   ├── decomposition/# 스킬 분해 및 도구 생성
│   ├── devtools/     # Trace/Span 타임라인 + RL Training 패널
│   ├── approval/     # 승인 프로세스 인터페이스
│   ├── recommendation/ # 도구/모델 추천
│   ├── onboarding/   # WelcomeWizard / InteractiveTutorial
│   ├── help/         # 도움말 패널
│   ├── notification/ # 알림 컴포넌트
│   ├── proactive/    # 능동 제안
│   ├── llm-wiki/     # LLM Wiki 컴포넌트
│   ├── wiki/         # Wiki 컴포넌트
│   ├── fine-tune/    # 파인튜닝 인터페이스
│   ├── trace/        # Trace 컴포넌트
│   ├── style/        # 스타일/테마
│   ├── shared/       # 공유 컴포넌트(ErrorBoundary / PageContextProvider)
│   └── common/       # 공용 컴포넌트(Icon 등)
│
├── stores/           # Zustand 상태 관리(82개 store)
│   ├── domain/       # 9개 핵심 비즈니스 store(대화/스트림/압축/환경설정/멀티모델 등)
│   ├── feature/      # 61개 기능 모듈 store(에이전트/워크플로우/지식 베이스/스킬/게이트웨이/메모리/터미널 등)
│   ├── shared/       # 8개 크로스 컴포넌트 공유 store(UI/탭/워크스페이스/백엔드 상태 등)
│   └── devtools/     # 4개 개발자 도구 store
│
├── hooks/            # React Hooks(단축키/명령 팔레트/반응형/스크롤바/테마/Avatar 등)
├── lib/              # 유틸리티 함수 라이브러리(invoke/pageRegistry/shortcuts/skillLifecycle/
│                     #   chartGenerator/codeExecutor/tokenEstimator/workflowLayout 등 45+ 모듈)
├── types/            # TypeScript 타입 정의
├── theme/            # Shadcn 테마 엔진
├── i18n/             # 11개 언어 번역 파일(zh-CN/zh-TW/en-US/ja/ko/fr/de/es/ru/hi/ar)
├── constants/        # 상수 및 기능 스위치
└── sdk/              # ACP 클라이언트 SDK(TypeScript + Python)
```

### 기능 플래그

프로젝트는 `featureFlags.ts`를 통해 점진적 기능 출시를 관리합니다:

| 플래그              | 상태 | 설명                                   |
| ------------------- | ---- | -------------------------------------- |
| `AGENT_IN_THE_LOOP` | ✅   | 전역 Agent 패널 + 페이지 컨텍스트 주입 |
| `DYNAMIC_UI`        | ✅   | 동적 UI 구축 엔진                      |
| `SELF_EVOLUTION_UI` | ❌   | 자기 진화 프론트엔드 컨트롤 플레인     |
| `NL_EXTENSION`      | ❌   | 자연어 기반 동적 비즈니스 확장         |

### Tauri 플러그인

| 플러그인            | 용도                |
| ------------------- | ------------------- |
| `autostart`         | 자동 시작           |
| `clipboard-manager` | 클립보드 읽기/쓰기  |
| `dialog`            | 파일 선택 대화상자  |
| `fs`                | 파일 시스템 접근    |
| `global-shortcut`   | 전역 단축키 등록    |
| `notification`      | 시스템 알림         |
| `opener`            | 외부 링크/파일 열기 |
| `process`           | 프로세스 관리       |
| `updater`           | 자동 업데이트       |

---

## 데이터 디렉터리

```
~/.axagent/                    # 앱 설정
├── axagent.db                 # SQLite 메인 데이터베이스(SeaORM)
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

### 환경 요구 사항

- [Node.js](https://nodejs.org/) 20+
- [Rust](https://www.rust-lang.org/) 1.75+(edition 2024)
- [npm](https://www.npmjs.com/) 10+
- Windows: [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)(MSVC + Windows SDK)
- macOS: Xcode Command Line Tools
- Linux: `build-essential` + `libwebkit2gtk-4.1-dev` + `libssl-dev`

### 개발

```bash
git clone https://github.com/polite0803/AxAgent.git
cd AxAgent
npm install
npm run tauri dev      # 개발 모드(프론트엔드 Vite HMR + Tauri 창)
```

### 빌드

```bash
npm run tauri build    # 데스크톱 프로덕션 빌드

npm run tauri:android:build   # Android 빌드
npm run tauri:ios:build       # iOS 빌드
```

데스크톱 빌드 산출물은 `src-tauri/target/release/`에 위치합니다.

### 테스트

```bash
npm run test           # 프론트엔드 단위 테스트(Vitest watch)
npm run test:run       # 프론트엔드 단위 테스트(단일 실행)
npm run test:e2e       # E2E 테스트(Playwright)

# Rust 백엔드 테스트
cd src-tauri && cargo test

# 타입 검사 & Lint
npm run typecheck
cd src-tauri && cargo clippy -- -D warnings
npm run format         # dprint 포맷팅
npm run lint:eslint    # ESLint 검사
npm run contracts      # API 계약 검사

# CI 전체 검사
npm run ci:check
```

### 자주 사용하는 스크립트

| 명령                     | 용도                         |
| ------------------------ | ---------------------------- |
| `npm run bump`           | 버전 번호 업그레이드(대화형) |
| `npm run docs`           | TypeDoc 문서 생성            |
| `npm run skill:create`   | 새 스킬 스캐폴드 생성        |
| `npm run skill:validate` | 스킬 정의 검증               |
| `npm run check:types`    | 타입 일관성 검사             |

---

## 지원 플랫폼

| 플랫폼  | 아키텍처                              |
| ------- | ------------------------------------- |
| Windows | x86_64, ARM64                         |
| macOS   | Apple Silicon (arm64), Intel (x86_64) |
| Linux   | x86_64, ARM64                         |
| Android | arm64-v8a, armeabi-v7a, x86_64        |
| iOS     | arm64                                 |

---

## 오픈소스 라이선스

이 프로젝트는 [AGPL-3.0-only](LICENSE) 라이선스로 오픈소스로 제공됩니다.

---

## 감사의 말

AxAgent는 많은 훌륭한 오픈소스 프로젝트 위에 구축되었습니다:

- [Tauri](https://tauri.app/) — 크로스 플랫폼 데스크톱 프레임워크
- [React](https://react.dev/) + [Ant Design](https://ant.design/) — 프론트엔드 UI
- [SeaORM](https://www.sea-ql.org/SeaORM/) — Rust ORM
- [sqlite-vec](https://github.com/asg017/sqlite-vec) — 벡터 검색
- [candle](https://github.com/huggingface/candle) — 로컬 임베딩 추론
- [rmcp](https://github.com/nicholasxjy/rmcp) — Rust MCP SDK
- [ReactFlow](https://reactflow.dev/) — 시각적 워크플로우 편집기
- [axum](https://github.com/tokio-rs/axum) — HTTP 프레임워크
- [Monaco Editor](https://microsoft.github.io/monaco-editor/) — 코드 편집기
- [xterm.js](https://xtermjs.org/) — 터미널 에뮬레이터
- [Zustand](https://zustand.docs.pmnd.rs/) — 상태 관리
- [Framer Motion](https://www.framer.com/motion/) — 애니메이션 라이브러리
- [Recharts](https://recharts.org/) — 차트 라이브러리
