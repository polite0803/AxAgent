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

**AxAgent**는 **Windows / macOS / Linux / Android / iOS** 5개 플랫폼을 지원하는 오픈소스 크로스 플랫폼 AI 어시스턴트 데스크톱 클라이언트입니다. 단순한 채팅 인터페이스가 아니라 ReAct 에이전트 엔진, 시각적 워크플로우 오케스트레이션, 로컬 RAG 지식 베이스, MCP 프로토콜 확장, 멀티 모델 통합 게이트웨이, 브라우저 자동화, 컴퓨터 제어 등을 통합하여 일상적인 개발, 연구, 지식 관리, 자동화 작업을 위한 AI 워크스테이션으로 기능합니다.

> **언어**: [简体中文](./README.md) | [English](./README-EN.md) | [繁體中文](./README-ZH-TW.md) | [日本語](./README-JA.md) | [한국어](./README-KO.md) | [Français](./README-FR.md) | [Deutsch](./README-DE.md) | [Español](./README-ES.md) | [Русский](./README-RU.md) | [हिन्दी](./README-HI.md) | [العربية](./README-AR.md)

---

## 프로젝트 방향성

AxAgent는 세 가지 핵심 문제를 해결합니다:

1. **멀티 모델 통합 스케줄링**: 단일 인터페이스에서 OpenAI, Anthropic Claude, Google Gemini, Ollama 로컬 모델 및 모든 OpenAI 호환 API를 동시에 사용하며, 멀티 키 로테이션, 지능형 모델 라우팅, 스트리밍 비교 지원
2. **AI 능력의 도구화**: AI를 "대화"에서 "실행"으로 확장——47개 이상의 내장 도구, 시각적 워크플로우, MCP 확장, 브라우저 자동화, 컴퓨터 제어를 통해 AI가 파일 조작, 코드 실행, Git 관리, 작업 스케줄링을 직접 수행
3. **로컬 우선 데이터 주권**: AI 대화, 지식 베이스, 메모리, 설정 파일은 모두 로컬 SQLite 데이터베이스에 저장되며, API 키는 AES-256-GCM으로 암호화. 서드파티 클라우드 서비스 없이도 핵심 기능 실행 가능

---

## 핵심 기능

### 멀티 모델 엔진

- **9개 제공자 어댑터**: OpenAI (Chat Completions + Responses + Realtime), Anthropic Claude, Google Gemini, Ollama (GGUF 관리 포함), OpenClaw, Hermes 및 모든 OpenAI 호환 API
- **멀티 키 로테이션**: 동일 제공자에 여러 API 키를 구성하고 할당량에 따라 자동 로테이션하여 단일 키 속도 제한 중단 방지
- **지능형 라우팅**: 작업 유형(코드 리뷰/요약/번역/일반)에 따라 가장 적합한 모델을 자동 선택, 사용자 정의 라우팅 규칙 지원
- **제공자 상태 모니터링**: 각 제공자의 성공률, 지연 시간, 가용 상태를 실시간 추적, 계층형 자동 강등(ProviderTier)
- **AI 이미지 생성**: DALL-E 3 및 Flux (Replicate) 다중 크기 프리셋
- **실시간 음성**: OpenAI Realtime API 기반 WebSocket 음성 대화, 중단 및 스트리밍 전사 지원

### 에이전트 시스템

전체 에이전트 시스템은 **ReAct (Reasoning + Acting) 엔진** 위에 구축되며, 다음과 같은 실제 구현된 하위 시스템을 포함합니다:

- **계층형 플래너** (`hierarchical_planner`): 복잡한 작업을 종속성이 있는 Phase → Task 구조화 계획으로 분해하여 DAG 토폴로지 실행으로 컴파일
- **심층 연구** (`deep_research`): 다중 소스 검색 오케스트레이션(검색 계획 (`search_planner`), 검색 실행 (`search_orchestrator`), 콘텐츠 종합 (`content_synthesizer`), 인용 추적 (`citation_tracker`))
- **팩트 체커** (`fact_checker`): AI 기반 사실 검증(소스 분류기 (`source_classifier`), 소스 검증기 (`source_validator`), 신뢰도 평가기 (`credibility_evaluator`))
- **사고의 나무** (`tree_of_thoughts`): 다중 경로 추론 탐색, 분기 평가 및 백트래킹
- **리플렉터** (`reflector`): 작업 실행 후 자체 평가 및 개선 제안 생성
- **자체 검증기** (`self_verifier`): 추론 결과 자동 검증, 순환 감지 (`cycle_detector`)로 무한 추론 방지
- **오류 복구** (`error_recovery_engine`): 오류 유형 분류 → 복구 전략 선택 → 자동 재시도 또는 계획 조정, 지수 백오프 지원
- **A/B 테스트** (`ab_testing`): 서로 다른 추론 전략 비교 평가
- **평가 시스템** (`evaluator`): 내장 벤치마크 프레임워크(데이터셋, 메트릭, 보고서 생성)
- **LoRA 파인튜닝** (`fine_tune`): 내장 학습 파이프라인, LoRA 어댑터 관리
- **RL 최적화기** (`rl_optimizer`): 경험 피드백 기반 정책 강화 학습(경험 재생, 정책 그래디언트)
- **도구 추천기** (`tool_recommender`): 컨텍스트 기반 도구 사용 패턴 분석 및 추천

**멀티 에이전트 협업**:

- 마스터-슬레이브 조정 아키텍처 (`coordinator`), 하위 에이전트 병렬 실행, 종속성 인식 스케줄링
- 에이전트 간 정보 교환을 위한 공유 블랙보드 (`shared_blackboard`)
- 대립적 토론 모드, Pro/Con 라운드 및 논점 강도 점수
- Swarm 클러스터 모드, 다중 프로세스 에이전트 클러스터(권한 동기화 및 자동 재연결 지원)
- 능동적 모드 (`proactive_mode`): 에이전트가 능동적으로 제안 및 작업 시작 가능

**컴퓨터 제어**: AI 구동 마우스 클릭, 키보드 입력, 화면 스크롤, 3단계 권한 수준(기본/편집 수락/전체 액세스), 샌드박스 경로 격리

**브라우저 자동화**: CDP 프로토콜을 통해 브라우저 제어, 탐색, 스크린샷, 클릭, 양식 작성, 텍스트 추출, 페이지 상태 모니터링 지원

### 스킬 시스템

- **스킬 마켓플레이스**: 커뮤니티 스킬 탐색 및 설치
- **AI 지원 생성**: 자연어 제안에서 스킬 구조 자동 생성
- **스킬 진화** (`evolution_engine`): 실행 피드백 기반 스킬 자동 분석 및 개선
- **의미 매칭** (`skill`): 대화 컨텍스트에 따른 관련 스킬 의미 매칭, 자동 추천
- **스킬 분해** (`skill_decomposition`): 복잡한 작업을 원자 스킬 조합으로 자동 분해
- **생성 도구** (`generated_tool`): AI가 새 도구 생성 및 등록
- **샌드박스 실행** (`sandbox`): 스킬을 격리된 샌드박스 환경에서 안전하게 실행

### 시각적 워크플로우

ReactFlow 12 기반 드래그 앤 드롭 DAG 워크플로우 편집기:

- **17개 노드 유형**: 트리거, 에이전트, LLM 호출, 조건 분기, 병렬 포크, 루프, 병합, 지연, 도구 호출, 코드 실행, 하위 워크플로우, 벡터 검색, 문서 구문 분석, 검증, 종료, 비즈니스 규칙, 에이전트 역할
- **Kahn 토폴로지 정렬 실행**: 순환 종속성 자동 감지, 병렬 파이프라인 스케줄링
- **내장 템플릿**: 코드 리뷰, 버그 수정, 문서 생성, 테스트, 리팩토링, 탐색, 성능 분석, 보안 감사, 기능 개발
- **YAML 직렬화**: 워크플로우 정의 YAML 형식 가져오기/내보내기 지원
- **버전 관리**: 워크플로우 템플릿 버전 제어
- **AI 지원**: AI 지원 워크플로우 설계 및 노드 추천

### 지식 관리

- **멀티 지식 베이스 RAG**: 문서 업로드 → 자동 구문 분석(PDF/DOCX/XLSX/PPTX/TXT) → 청크 분할 → 벡터 인덱싱
- **하이브리드 검색**: 벡터 유사도(sqlite-vec + candle 로컬 임베딩) + BM25 전체 텍스트 검색(FTS5), 하이브리드 순위
- **Self-RAG**: 자체 검색 증강 생성, 검색 결과 자동 반성 및 검증
- **재순위 지정**: Cross-encoder 결과 재순위 지정으로 정밀도 향상
- **지식 그래프**: 엔티티 추출 (`EntityExtractor`) → 관계 구축 → 시각화 그래프
- **파일 감시**: `notify` 기반 실시간 파일 변경 감시, 자동 증분 인덱싱
- **LLM Wiki**: AI 지원 Wiki 컴파일러 및 검증기, Wiki 크로핑 브라우저 확장 지원

### 메모리 시스템

- **멀티 네임스페이스 메모리**: 프로젝트/주제별 격리, 수동 입력 및 AI 자동 추출 지원
- **영속성 통합**: Honcho 및 Mem0 폐쇄 루프 메모리
- **사용자 프로필** (`user_profile` / `profile`): 코드 스타일(들여쓰기/명명/주석), 기술 스택 선호도, 커뮤니케이션 스타일 자동 학습
- **스타일 전송** (`style`): 코드 스타일 특징 추출 → AI 생성 코드에 적용
- **꿈 통합** (`dream`): 백그라운드에서 메모리 조각 및 행동 패턴 자동 통합, 구조화된 지식 생성
- **프로젝트 메모리** (`project_memory`): 프로젝트 차원 컨텍스트 영속화

### API 게이트웨이

`axum` 기반 내장 HTTP + WebSocket 게이트웨이 서버:

- **호환 엔드포인트**: OpenAI `/v1/chat/completions`, Claude Messages API, Gemini API 및 OpenAI Responses와 Realtime WebSocket
- **키 관리**: 액세스 키 생성, 취소, 활성화/비활성화, 만료 시간 설정
- **사용량 추적**: 키, 제공자, 날짜별 요청량 및 토큰 소비 통계, Prometheus 메트릭 내보내기
- **속도 제한**: `governor` 기반 토큰 버킷 알고리즘, 구성 가능한 속도 제한 정책
- **SSL/TLS**: 내장 자체 서명 인증서 (`rcgen`), 사용자 정의 인증서 지원
- **외부 링크**: Claude CLI, OpenCode 등 외부 도구 원클릭 통합, API 키 자동 동기화
- **실시간 티켓**: HMAC 기반 임시 인증 티켓, WebSocket 실시간 연결 안전 전달

### 메시징 플랫폼 통합

`rt-messaging` 크레이트로 구현된 메시징 플랫폼 게이트웨이, 다음 지원:

DingTalk, Feishu, QQ, Slack, WeChat, WhatsApp, Telegram, Discord

Webhook 메시지 수신, 명령 구문 분석, AI 응답 자동 중계 지원.

### 도구 시스템

47개 내장 도구, 모두 `Tool` 트레이트를 통해 등록:

| 범주         | 도구                                                                                                                                                                                                       |
| ------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 파일 작업    | `file_read`, `file_write`, `file_edit`, `file_system` (목록/검색/메타데이터)                                                                                                                               |
| 코드 실행    | `bash`, `repl`                                                                                                                                                                                             |
| 검색         | `grep`, `glob`                                                                                                                                                                                             |
| 브라우저     | `browser` (CDP 제어)                                                                                                                                                                                       |
| 컴퓨터 제어  | `computer_use` (마우스/키보드/스크린샷)                                                                                                                                                                    |
| Web          | `web_search`, `web_fetch`                                                                                                                                                                                  |
| 지식 베이스  | `knowledge`, `document` (문서 구문 분석)                                                                                                                                                                   |
| Git          | `git` (commit/push/branch/diff)                                                                                                                                                                            |
| 개발 도구    | `lsp` (언어 서버 프로토콜), `workspace`                                                                                                                                                                    |
| 작업 관리    | `plan`, `task_system`, `todo_write`, `cron`                                                                                                                                                                |
| 푸시 알림    | `push_notification`, `messaging`                                                                                                                                                                           |
| 데이터베이스 | `database`                                                                                                                                                                                                 |
| 스토리지     | `storage`                                                                                                                                                                                                  |
| 기타         | `agent`, `agent_memory`, `context`, `export`, `integration`, `media`, `media_delivery`, `migration_tool`, `monitor`, `obsidian`, `ocr`, `personality`, `shared_path`, `system_info`, `testing`, `worktree` |

### MCP 프로토콜

`rmcp` 크레이트 기반 완전한 MCP (Model Context Protocol) 구현:

- **전송 계층**: stdio 하위 프로세스 + Streamable HTTP + WebSocket
- **OAuth 인증**: MCP 서버 OAuth 인증 흐름 지원
- **도구 발견**: MCP 서버가 노출하는 도구 자동 발견 및 등록
- **MCP 관리자**: 서버 수명 주기 관리, 상태 확인, 자동 재연결

### 플러그인 시스템

OpenClaw 호환 3계층 플러그인 아키텍처(내장/번들/외부), 다음 지원:

- npm 패키지 설치, 검색 및 설치를 위한 내장 마켓플레이스 UI
- 플러그인 매니페스트 정의, 권한 선언, 샌드박스 격리 실행
- 사용자 정의 도구 등록, 에이전트 제공자, Hook 차단
- 스킬 설치 프로그램: 플러그인 패키지에서 스킬을 스킬 시스템으로 설치

### 보안

- **AES-256-GCM 암호화**: API 키 및 민감한 설정의 로컬 암호화 저장 (`crypto` 크레이트)
- **프롬프트 주입 방어**: 4계층 방어 파이프라인 (`prompt-guard`)——패턴 감지 → 구분 기호 이스케이프 → XML 래퍼 → 신뢰 태그, 세션, 프롬프트 구성, Git, RAG 전체 체인 통합
- **SSRF 방어** (`ssrf_guard`): URL 보안 검사, 내부 네트워크 주소 요청 차단
- **콘텐츠 필터링** (`content_filter`): 다중 유형 콘텐츠 보안 필터링
- **속도 제한** (`rate_limiter`): 도구 호출 및 API 요청 토큰 버킷 속도 제한
- **서킷 브레이커** (`circuit_breaker`): 연속 실패 시 자동 차단, 시스템 안정성 보호
- **액세스 제어** (`tool_access`): 정책 기반 도구 액세스 권한 제어
- **샌드박스 격리**: 에이전트 및 스킬 실행 환경 격리

### 개발자 경험

- **분산 추적** (`telemetry`): OpenTelemetry 통합, Span/Trace 시각화 지원
- **원격 측정** (`telemetry`): 구조화된 로깅, 런타임 메트릭, 성능 이벤트 수집
- **리플레이 디버깅**: 에이전트 실행 궤적 기록 (`trajectory_recorder`) 및 리플레이
- **DevTools 패널**: 프론트엔드 내장 Trace/Span 타임라인 뷰어
- **벤치마크 프레임워크**: Criterion 벤치마크 (tool_exec / llm_call / search), SWE-bench 및 Terminal-bench 평가

### 데스크톱 및 모바일 경험

- **반응형 레이아웃**: CSS 중단점으로 데스크톱/태블릿/모바일 적응(600px/900px)
- **11개 언어**: 간체 중국어, 번체 중국어, 영어, 일본어, 한국어, 프랑스어, 독일어, 스페인어, 러시아어, 힌디어, 아랍어
- **테마 엔진** (`rt-theme`): 다크/라이트 테마, 시스템 따르기 또는 수동 전환, Ant Design 6 심층 커스터마이징
- **Monaco 편집기**: 내장 코드 편집기, 구문 강조, 차이 미리보기, 다국어 지원
- **xterm.js 터미널**: 내장 터미널 에뮬레이터, WebLinks, Unicode 11, 검색 지원
- **D2 / Mermaid / ECharts**: 아키텍처 다이어그램, 순서도, 대화형 차트 렌더링
- **세션 공유**: 원클릭 공유 링크 생성, 액세스 권한 구성 가능
- **시스템 트레이 + 전역 단축키 + 자동 시작**: 방해 없는 백그라운드 실행
- **자동 업데이트**: GitHub Releases 버전 업데이트 자동 감지
- **프록시 지원**: HTTP 및 SOCKS5 프록시 구성
- **클라우드 작업 공간**: S3 및 WebDAV 스토리지 동기화, 충돌 감지 및 양방향 동기화

### 모바일

- Android APK/AAB (arm64-v8a, armeabi-v7a, x86_64)
- iOS IPA (arm64)
- 모바일 전용 적응: 안전 영역 적응, 하단 탐색 바, Drawer 탐색

---

## 기술 아키텍처

### 기술 스택

| 계층                  | 기술                                     |
| --------------------- | ---------------------------------------- |
| 데스크톱 프레임워크   | Tauri 2.11                               |
| 프론트엔드 프레임워크 | React 19 + TypeScript 6                  |
| UI 라이브러리         | Ant Design 6 + TailwindCSS 4             |
| 상태 관리             | Zustand 5                                |
| 라우팅                | React Router 7                           |
| 코드 편집기           | Monaco Editor                            |
| 터미널                | xterm.js 6                               |
| 워크플로우 편집기     | ReactFlow 12                             |
| 차트                  | D2 + Mermaid + Recharts + ECharts        |
| 가상 스크롤           | @tanstack/react-virtual + react-virtuoso |
| 드래그 앤 드롭        | @dnd-kit                                 |
| Markdown 렌더링       | markstream-react + stream-markdown       |
| 국제화                | i18next + react-i18next                  |
| 빌드 도구             | Vite 8                                   |
| 테스트                | Vitest + Playwright + cargo-nextest      |
| 포맷팅                | dprint (TS/JSON/Markdown/TOML) + rustfmt |
| Lint                  | ESLint + Oxlint + Clippy + cargo-deny    |

### 백엔드 아키텍처: Harness 의존성 주입 패턴

백엔드는 Rust 워크스페이스 아키텍처를 사용하며 **32개 크레이트**를 포함하고 **Harness 아키텍처 패턴**을 따릅니다:

```
모든 크레이트는 axagent-harness가 정의한 트레이트 인터페이스를 통해 분리되며,
런타임에 axagent-runtime이 종속성을 조립하고 주입합니다.

종속성 방향: 구체 구현 → harness ← 호출자
```

**harness**는 아키텍처의 초석입니다——비즈니스 로직도 구체 구현도 없으며, 트레이트 정의, 순수 데이터 DTO, 상수, 통합 오류 유형만 포함합니다. 다른 모든 크레이트가 이에 의존하며, 자체는 다른 axagent-* 크레이트에 의존하지 않습니다.

```
src-tauri/crates/
├── harness/          # 아키텍처 초석 — 트레이트 인터페이스, DTO, 통합 오류 유형, DI 계약
│                     #   200개 이상 트레이트 정의: Agent/Provider/Tool/RAG/스토리지/
│                     #   MCP/플러그인/보안/관찰 가능성/메모리/학습/브라우저/메시징 등
│
├── entities/         # SeaORM 엔티티 모델
├── dao/              # 데이터 액세스 계층(CRUD)
├── migration/        # 데이터베이스 마이그레이션
│
├── crypto/           # AES-256-GCM 암호화/복호화 및 키 관리
├── credential/       # 자격 증명 보안 저장(API 키 등)
├── storage/          # 파일 스토리지 추상화(로컬/S3/WebDAV), ZIP 읽기/쓰기 지원
├── cache/            # 범용 캐시 계층(메모리)
├── disk-cache/       # 디스크 파일 수준 캐시
├── search/           # 검색 엔진(FTS5 + sqlite-vec + candle 임베딩)
├── document-parser/  # 문서 텍스트 추출(PDF/DOCX/XLSX/PPTX)
├── kit/              # 범용 유틸리티 툴킷 — 경로/인코딩/해시/날짜 등
│
├── runtime-core/     # 런타임 공통 유형, 구성 상수
├── runtime/          # 런타임 서비스 오케스트레이션 — 전체 30개 이상 크레이트 조립, Harness DI의 런타임 컨테이너
│                     #   관리: 세션/터미널/Webhook/속도 제한/권한/SSRF/이벤트 버스/상태
├── rt-workflow/      # 워크플로우 엔진 — DAG 오케스트레이션, 노드 실행기, YAML 직렬화
├── rt-messaging/     # 메시징 플랫폼 게이트웨이 — DingTalk/Feishu/QQ/Slack/WeChat/WhatsApp/Telegram/Discord
├── rt-webhook/       # 범용 Webhook 서버 및 이벤트 디스패치
├── rt-dashboard/     # 대시보드 플러그인 프레임워크
├── rt-theme/         # 테마 엔진 — 다크/라이트 전환 로직
│
├── agent/            # AI 에이전트 코어 — 80개 이상 모듈
│                     #   ReAct 엔진/계층형 계획/심층 연구/팩트 체크/사고의 나무/반성/
│                     #   자체 검증/오류 복구/RL 최적화/LoRA 파인튜닝/평가/도구 추천/A-B 테스트/
│                     #   조정기/블랙보드/비전 파이프라인/Web 검색/학술 검색/Wiki 컴파일 등
│
├── orchestrator/     # 에이전트 오케스트레이션 — 멀티 에이전트 스케줄링, DAG 분해, 동적 하위 그래프 실행
├── providers/        # 모델 제공자 어댑터 — OpenAI/Anthropic/Gemini/Ollama/
│                     #   OpenClaw/Hermes/이미지 생성(DALL-E/Flux)/Realtime/Responses
├── tools/            # 도구 시스템 — Tool 트레이트/레지스트리/오케스트레이션/스트리밍/샌드박스/47개 이상 내장 도구
├── gateway/          # API 게이트웨이 — axum HTTP/WS 서버, OAuth, 속도 제한, Prometheus
├── mcp/              # MCP 프로토콜 — stdio + Streamable HTTP, rmcp 기반
├── trajectory/       # 학습 시스템 — 메모리/스킬 진화/사용자 프로필/꿈 통합
├── plugins/          # 플러그인 시스템 — OpenClaw 호환, npm 패키지 설치, 마켓플레이스
├── telemetry/        # 관찰 가능성 — OpenTelemetry, 구조화된 로깅, 런타임 메트릭
├── prompt-guard/     # 프롬프트 주입 방어 — L1-L4 다단계 감지 파이프라인
├── npm/              # npm 레지스트리 클라이언트
└── schema-gen/       # 데이터베이스 스키마 생성 도구
```

### 프론트엔드 아키텍처

```
src/
├── pages/            # 22개 페이지
│   ├── ChatPage          # 메인 채팅 인터페이스
│   ├── WorkflowPage      # 워크플로우 편집기
│   ├── GatewayPage       # API 게이트웨이 관리
│   ├── KnowledgeHubPage  # 지식 베이스 관리
│   ├── MemoryPage        # 메모리 관리
│   ├── SkillsPage        # 스킬 마켓플레이스
│   ├── SettingsPage      # 설정 패널
│   ├── DashboardPage     # 데이터 대시보드
│   ├── TerminalPage      # 터미널
│   ├── FilesPage         # 파일 관리
│   ├── GatewayLinkPage   # 외부 링크 관리
│   ├── LinkPage          # 통합 링크
│   ├── WikiEditorPage    # Wiki 편집기
│   ├── WikiEditPage      # Wiki 편집
│   ├── WikiGraphPage     # Wiki 지식 그래프
│   ├── FineTunePage      # LoRA 파인튜닝
│   ├── PersonaPage       # 페르소나 관리
│   ├── QuickBarPage      # 빠른 바
│   ├── IngestPage        # 문서 수집
│   ├── WorkflowMarketplace # 워크플로우 마켓플레이스
│   ├── DynamicUIManagerPage # 동적 UI 관리
│   └── DynamicPageViewer    # 동적 페이지 뷰어
│
├── components/       # 24개 모듈, 200개 이상 컴포넌트
│   ├── chat/         # 채팅 인터페이스(메시지 스트림/입력/첨부/도구 호출/아티팩트/사고 블록 등)
│   ├── workflow/     # 워크플로우 편집기(노드/엣지/패널/템플릿/AI 지원)
│   ├── gateway/      # API 게이트웨이 관리 UI
│   ├── settings/     # 설정 패널(40개 이상 하위 컴포넌트)
│   ├── skill/        # 스킬 편집기 및 렌더러
│   ├── benchmark/    # 벤치마크 패널
│   ├── decomposition/# 스킬 분해 및 도구 생성
│   ├── devtools/     # Trace/Span 타임라인
│   ├── layout/       # 레이아웃(타이틀 바/사이드바/명령 팔레트)
│   └── ...
│
├── stores/           # 62개 Zustand 스토어
│   ├── domain/       # 핵심 비즈니스 상태
│   ├── feature/      # 기능 모듈 상태(44개)
│   └── devtools/     # 개발자 도구 상태
│
├── hooks/            # React Hooks
├── lib/              # 유틸리티 함수 + Web Workers
├── types/            # TypeScript 유형 정의
├── sdk/              # 외부 통합 SDK
└── i18n/             # 11개 언어 번역 (zh-CN/zh-TW/en-US/ja/ko/fr/de/es/ru/hi/ar)
```

---

## 데이터 디렉토리

```
~/.axagent/                    # 애플리케이션 구성
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

### 요구 사항

- [Node.js](https://nodejs.org/) 20+
- [Rust](https://www.rust-lang.org/) 1.75+, edition 2024
- [npm](https://www.npmjs.com/) 10+
- Windows: [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) (MSVC + Windows SDK)
- macOS: Xcode Command Line Tools
- Linux: `build-essential` + `libwebkit2gtk-4.1-dev` + `libssl-dev`

### 빌드

```bash
git clone https://github.com/polite0803/AxAgent.git
cd AxAgent
npm install
npm run tauri dev      # 개발 모드
npm run tauri build    # 프로덕션 빌드
```

빌드 결과물은 `src-tauri/target/release/`에 있습니다.

### 테스트

```bash
npm run test           # 프론트엔드 단위 테스트 (Vitest watch)
npm run test:run       # 프론트엔드 단위 테스트 (단일 실행)
npm run test:e2e       # E2E 테스트 (Playwright)

# Rust 백엔드 테스트
cd src-tauri && cargo nextest run
cd src-tauri && cargo test

# 유형 검사 & Lint
npm run typecheck
cd src-tauri && cargo clippy -- -D warnings
npm run format

# CI 전체 검사
npm run ci:check
```

---

## 플랫폼 지원

| 플랫폼  | 아키텍처                                    |
| ------- | ------------------------------------------- |
| Windows | x86_64, ARM64                               |
| macOS   | Apple Silicon (arm64), Intel (x86_64)       |
| Linux   | x86_64, ARM64                               |
| Android | arm64-v8a, armeabi-v7a, x86_64 (에뮬레이터) |
| iOS     | arm64                                       |

---

## 라이선스

이 프로젝트는 [AGPL-3.0-only](LICENSE) 라이선스 하에 오픈소스로 공개됩니다.

---

## 감사의 글

AxAgent는 다음을 포함한 수많은 훌륭한 오픈소스 프로젝트 위에 구축되었습니다:

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
