[**English**](./README-EN.md) | [简体中文](./README.md) | [繁體中文](./README-ZH-TW.md) | [日本語](./README-JA.md) | **한국어** | [Français](./README-FR.md) | [Deutsch](./README-DE.md) | [Español](./README-ES.md) | [Русский](./README-RU.md) | [हिन्दी](./README-HI.md) | [العربية](./README-AR.md)

[![AxAgent](https://github.com/polite0803/AxAgent/blob/main/src/assets/image/logo.png?raw=true)](https://github.com/polite0803/AxAgent)

<p align="center">
  <a href="https://www.producthunt.com/products/axagent?embed=true&amp&amp&utm_source=badge-featured&amp&amp;&amp;#10;&amp;amp&amp&amp;;utm_medium=badge&amp&amp;#10&amp&amp;;utm_campaign=badge-axagent" target="_blank" rel="noopener noreferrer"><img alt="AxAgent - Lightweight, high-perf cross-platform AI desktop client | Product Hunt" width="250" height="54" src="https://api.producthunt.com/widgets/embed-image/v1/featured.svg?post_id=1118403&amp;theme=light&amp;t=1775627359538"></a>
</p>

<p align="center">
  <strong>크로스 플랫폼 AI 데스크톱 클라이언트 | 멀티 에이전트 협업 | 로컬 우선</strong>
</p>

<p align="center">
  <a href="https://github.com/polite0803/AxAgent/releases" target="_blank">
    <img src="https://img.shields.io/github/v/release/polite0803/AxAgent?style=flat-square" alt="Release">
  </a>
  <a href="https://github.com/polite0803/AxAgent/actions" target="_blank">
    <img src="https://img.shields.io/github/actions/workflow/status/polite0803/AxAgent/release.yml?style=flat-square" alt="Build">
  </a>
  <img src="https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-blue?style=flat-square" alt="Platform">
  <img src="https://img.shields.io/badge/license-AGPL--3.0-green?style=flat-square" alt="License">
</p>

---

## AxAgent란?

AxAgent는 고급 AI 에이전트 기능과 풍부한 개발자 도구를 결합한 종합적인 크로스 플랫폼 AI 데스크톱 애플리케이션입니다. 멀티 프로바이더 모델 지원, 자율 에이전트 실행, 시각적 워크플로 오케스트레이션, 로컬 지식 관리 및 내장 API 게이트웨이를 제공합니다.

---

## 스크린샷

| 채팅 및 모델 선택 | 멀티 에이전트 대시보드 |
|:---:|:---:|
| ![](.github/images/s1-0412.png) | ![](.github/images/s5-0412.png) |

| 지식 베이스 RAG | 메모리 및 컨텍스트 |
|:---:|:---:|
| ![](.github/images/s3-0412.png) | ![](.github/images/s4-0412.png) |

| 워크플로 편집기 | API 게이트웨이 |
|:---:|:---:|
| ![](.github/images/s9-0412.png) | ![](.github/images/s10-0412.png) |

---

## 핵심 기능

### 🤖 AI 모델 지원

- **멀티 프로바이더 지원** — OpenAI, Anthropic Claude, Google Gemini, Ollama, OpenClaw, Hermes 및 모든 OpenAI 호환 API와 네이티브 통합
- **멀티 키 로테이션** — 각 프로바이더에 여러 API 키를 구성하고 자동 로테이션으로 비율 제한 분산
- **로컬 모델 지원** — Ollama 로컬 모델 및 GGUF/GGML 파일 관리를 완벽하게 지원
- **모델 관리** — 원격 모델 목록 가져오기, 사용자 지정 가능한 매개변수(temperature, max tokens, top-p 등)
- **스트리밍 출력** — 실시간 토큰 단위 렌더링, 접이식思考 블록(Claude 확장思考) 지원
- **멀티 모델 비교** — 여러 모델에 동시에 동일한 질문を送信하고 나란히 비교
- **함수 호출** — 지원되는 모든 프로바이더에 걸친 구조화된 함수 호출

### 🔐 AI 에이전트 시스템

에이전트 시스템은 정교한 아키텍처를 기반으로 구축되어 다음 기능을 제공합니다:

- **ReAct 추론 엔진** — 추론과 행동을 통합하고 자체 검증을 내장하여 작업 실행의 신뢰성 보장
- **계층적 플래너** — 복잡한 작업을 단계 및 의존성を持つ 구조화된 계획으로 분해
- **도구 레지스트리** — 동적 도구 등록, 의미적 버전 관리 및 충돌 감지 지원
- **컴퓨터 제어** — AI 제어 마우스 클릭, 키보드 입력, 화면 스크롤, 비전 모델 분석과 연계
- **화면 인식** — 스크린샷 캡처 및 비전 모델 분석으로 UI 요소 식별
- **3단계 권한 모드** — 기본(승인 필요), 편집 수락(자동 승인), 전체 액세스(프롬프트 없음)
- **샌드박스 격리** — 에이전트 작업은 지정된 작업 디렉토리로 엄격히 제한
- **도구 승인 패널** — 도구 호출 요청의 실시간 표시, 항목별 검토 지원
- **비용 추적** — 각 세션의 토큰 사용량 및 비용 통계 실시간 표시
- **일시 중지/재개** — 에이전트 실행을 언제든지 일시 중지하고 나중에 재개
- **체크포인트 시스템** — 크래시 복구 및 세션 재개를 위한 영속성 체크포인트
- **오류 복구 엔진** — 자동 오류 분류 및 복구 전략 실행

### 👥 멀티 에이전트 협업

- **하위 에이전트 조정** — 마스터-슬레이브 아키텍처로 여러 협업 에이전트 지원
- **병렬 실행** — 여러 에이전트가 작업을 병렬 처리, 의존성 인식 스케줄링 지원
- **적대적 디베이트** — Pro/Con 디베이트 라운드, 논점 강도 점수 매기기 및 반박 추적 지원
- **에이전트 역할** — 팀 협업을 위한 사전 정의된 역할(연구자, 플래너, 개발자, 검토자, 종합자)
- **에이전트 오케스트레이터** — 멀티 에이전트 팀을 위한 중앙 집중식 메시지 라우팅 및 상태 관리
- **통신 그래프** — 에이전트 상호작용 및 메시지 흐름의 시각적 표현

### ⭐ 스킬 시스템

- **스킬 마켓플레이스** — 커뮤니티 기여 스킬을 검색하고 설치할 수 있는 내장 마켓플레이스
- **스킬 생성** — 제안에서 자동으로 스킬 생성, Markdown 편집기 지원
- **스킬 진화** — 실행 피드백에 기반한 AI 구동 기존 스킬의 자동 분석 및 개선
- **스킬 매칭** — 의미적 매칭으로 대화 컨텍스트와 관련된 스킬 추천
- **원자 스킬** — 복잡한 워크플로로 구성 가능한 세분화된 스킬 구성 요소
- **스킬 분해** — 복잡한 작업을 자동으로 실행 가능한 원자 스킬로 분해
- **생성 도구** — AI가 자동으로 새로운 도구를 생성하고 등록하여 에이전트 능력 확장
- **스킬 허브** —集中的な 스킬 발견 및 구성 관리 인터페이스
- **스킬 허브 클라이언트** — 원격 스킬 허브와의 통합, 커뮤니티 공유 지원

### 🔄 워크플로 시스템

워크플로 엔진은 DAG 기반 작업 오케스트레이션 시스템을 구현합니다:

- **시각적 워크플로 편집기** — 노드 연결 및 구성을 지원하는 드래그 앤 드롭 워크플로 디자이너
- **풍부한 노드 유형** — 14가지 노드 유형: 트리거, 에이전트, LLM, 조건, 병렬, 루프, 병합, 지연, 도구, 코드, 원자 스킬, 벡터 검색, 문서 파서, 검증
- **워크플로 템플릿** — 내장 프리셋: 코드 리뷰, 버그 수정, 문서, 테스트, 리팩토링, 탐색, 성능, 보안, 기능 개발
- **DAG 실행** — Kahn 알고리즘による拓扑排序, 순환 감지 지원
- **병렬 디스패치** — 파이프라인 스타일 실행, 빠른 단계가 느린 단계를 기다리지 않음
- **재시도 정책** — 지수 백오프, 각 단계별 구성 가능한 최대 재시도 횟수
- **부분 완료** — 실패한 단계가 독립적인 하류 단계를 차단하지 않음
- **버전 관리** — 워크플로 템플릿 버전 관리, 롤백 지원
- **실행 기록** — 상세한 기록, 상태 추적 및 디버깅 지원
- **AI 지원** — AI 지원 워크플로 설계 및 최적화

### 📚 지식 및 메모리

- **지식 베이스(RAG)** — 멀티 지식 베이스 지원, 문서 업로드, 자동 분석, 청킹 및 벡터 인덱싱 지원
- **하이브리드 검색** — 벡터 유사성 검색과 BM25 전체 텍스트 순위 조합
- **리랭킹** — 교차 인코더 리랭킹으로 검색 정확도 향상
- **지식 그래프** — 지식 연결의 엔티티 관계 시각화
- **메모리 시스템** — 멀티 네임스페이스 메모리, 수동 입력 또는 AI 자동 추출 지원
- **폐쇄 루프 메모리** — Honcho 및 Mem0 영속성 메모리 프로바이더와의 통합
- **FTS5 전체 텍스트 검색** — 대화, 파일, 메모리 전체의 빠른 검색
- **세션 검색** — 모든 대화 세션 전체의 고급 검색
- **컨텍스트 관리** — 파일, 검색 결과, 지식 스니펫, 메모리, 도구 출력의 유연한 첨부

### 🌐 API 게이트웨이

- **로컬 API 서버** — 내장 OpenAI 호환, Claude 및 Gemini 인터페이스 서버
- **외부 링크** — 원클릭 Claude CLI, OpenCode 통합, API 키 자동 동기화
- **키 관리** — 생성, 취소, 활성화/비활성화, 설명이 있는 액세스 키 관리
- **사용량 분석** — 키, 프로바이더, 날짜별 요청량 및 토큰 사용량
- **SSL/TLS 지원** — 내장 자체 서명 인증서, 사용자 정의 인증서 지원
- **요청 로깅** — 모든 API 요청 및 응답의 완전한 기록
- **구성 템플릿** — Claude, Codex, OpenCode, Gemini의 사전 구축된 템플릿
- **실시간 API** — OpenAI 실시간 API 호환 WebSocket 이벤트 푸시
- **플랫폼 통합** — DingTalk, Feishu, QQ, Slack, WeChat, WhatsApp 지원

### 🔧 도구 및 확장

- **MCP 프로토콜** — 완전한 모델 컨텍스트 프로토콜 구현, stdio 및 HTTP/WebSocket 전송 지원
- **OAuth 인증** — MCP 서버의 OAuth 흐름 지원
- **내장 도구** — 파일 작업, 코드 실행, 검색 등의 종합적인 도구 세트
- **LSP 클라이언트** — 내장 언어 서버 프로토콜, 코드 완성 및 진단 지원
- **터미널 백엔드** — 로컬, Docker 및 SSH 터미널 연결 지원
- **브라우저 자동화** — CDP를 통한 브라우저 제어 기능 통합
- **UI 자동화** — 크로스 플랫폼 UI 요소 식별 및 제어
- **Git 도구** — 분기 감지 및 충돌 인식을 지원하는 Git 작업

### 📊 콘텐츠 렌더링

- **Markdown 렌더링** — 코드 하이라이트, LaTeX 수학, 표, 작업 목록의 완전한 지원
- **Monaco 코드 편집기** — 내장 편집기, 구문 하이라이트, 복사, 차이점 미리보기 지원
- **다이어그램 렌더링** — Mermaid 플로우차트, D2 아키텍처 다이어그램, ECharts 대화형 차트
- **아티팩트 패널** — 코드 스니펫, HTML 초안, React 구성 요소, Markdown 노트, 실시간 미리보기 지원
- **세 가지 미리보기 모드** — 코드(편집기), 분할(나란히), 미리보기(렌더링만)
- **세션 검사기** — 세션 구조의 트리 뷰, 빠른 탐색
- **인용 패널** — 소스 인용 추적 및 표시, 신뢰도 점수 매기기 지원

### 🛡️ 데이터 및 보안

- **AES-256 암호화** — API 키 및 민감한 데이터는 AES-256-GCM으로 암호화
- **분리 저장소** — 애플리케이션 상태는 `~/.axagent/`에, 사용자 파일은 `~/Documents/axagent/`에 저장
- **자동 백업** — 로컬 디렉토리 또는 WebDAV 저장소로 예약된 백업
- **백업 복원** — 원클릭으로 이전 백업에서 복원
- **내보내기 옵션** — PNG 스크린샷, Markdown, 일반 텍스트, JSON 형식
- **저장소 관리** — 시각적 디스크 사용량 표시 및 정리 도구

### 🖥️ 데스크톱 환경

- **테마 엔진** — 다크/라이트 테마, 시스템 따르기 또는 수동 기본 설정 지원
- **인터페이스 언어** — 12개 언어: 간체 중국어, 번체 중국어, 영어, 일본어, 한국어, 프랑스어, 독일어, 스페인어, 러시아어, 힌디어, 아랍어
- **시스템 트레이** — 백그라운드 서비스를 중단하지 않고 트레이로 최소화
- **항상 위에** — 다른 창보다 앞에 창 고정
- **전역 단축키** — 주 창을 호출하기 위한 사용자 정의 가능한 단축키
- **자동 시작** — 시스템 시작 시 선택적 실행
- **프록시 지원** — HTTP 및 SOCKS5 프록시 구성
- **자동 업데이트** — 자동 버전 확인 및 업데이트 프롬프트
- **명령 팔레트** — `Cmd/Ctrl+K` 빠른 명령 액세스

### 🔬 고급 기능

- **Cron 스케줄러** — 매일/매주/매월 템플릿 및 사용자 정의 cron 표현식을 통한 자동화된 작업 스케줄링
- **Webhook 시스템** — 도구 완료, 에이전트 오류, 세션 종료 알림의 이벤트 구독
- **사용자 프로파일링** — 코드 스타일, 명명 규칙, 들여쓰기, 주석 스타일, 커뮤니케이션 기본 설정의 자동 학습
- **RL 옵티마이저** — 도구 선택 및 작업 전략 최적화를 위한 강화 학습
- **LoRA 미세 조정** — LoRA를 사용한 로컬 교육으로 사용자 정의 모델 어댑테이션
- **능동적 제안** — 대화 내용 및 사용자 패턴에 기반한 컨텍스트 인식 힌트
- **사고 체인** — 에이전트 결정 추론의 시각화, 단계별 분해
- **오류 복구** — 자동 오류 분류, 근본 원인 분석 및 복구 제안
- **개발자 도구** — 디버깅 및 성능 분석을 위한 Trace, Span, 타임라인 시각화
- **벤치마크 시스템** — 점수 카드가 있는 작업 성능 평가 및 지표
- **스타일 전송** — 학습한 코드 스타일 기본 설정을 생성된 코드에 적용
- **대시보드 플러그인** — 사용자 정의 패널 및 위젯을 지원하는 확장 가능한 대시보드

---

## 기술 아키텍처

### 기술 스택

| 레이어 | 기술 |
|--------|------|
| **프레임워크** | Tauri 2 + React 19 + TypeScript |
| **UI** | Ant Design 6 + TailwindCSS 4 |
| **상태 관리** | Zustand 5 |
| **i18n** | i18next + react-i18next |
| **백엔드** | Rust + SeaORM + SQLite |
| **벡터 DB** | sqlite-vec |
| **코드 편집기** | Monaco Editor |
| **다이어그램** | Mermaid + D2 + ECharts |
| **터미널** | xterm.js |
| **빌드** | Vite + npm |

### Rust 백엔드 아키텍처

백엔드는 전문화된 crates로 구성된 Rust workspace로 구성됩니다:

```
src-tauri/crates/
├── agent/         # AI 에이전트 코어
│   ├── react_engine.rs       # ReAct 추론 엔진
│   ├── tool_registry.rs      # 동적 도구 등록
│   ├── coordinator.rs        # 에이전트 조정
│   ├── hierarchical_planner.rs # 작업 분해
│   ├── self_verifier.rs      # 출력 검증
│   ├── error_recovery_engine.rs # 오류 처리
│   ├── vision_pipeline.rs    # 화면 인식
│   └── fine_tune/            # LoRA 미세 조정
│
├── core/          # 코어 유틸리티
│   ├── db.rs               # SeaORM 데이터베이스
│   ├── vector_store.rs     # sqlite-vec 통합
│   ├── rag.rs             # RAG 추상화 레이어
│   ├── hybrid_search.rs    # 벡터 + FTS5 검색
│   ├── crypto.rs           # AES-256 암호화
│   └── mcp_client.rs       # MCP 프로토콜 클라이언트
│
├── gateway/       # API 게이트웨이
│   ├── server.rs          # HTTP 서버
│   ├── handlers.rs         # API 핸들러
│   ├── auth.rs            # 인증
│   └── realtime.rs        # WebSocket 지원
│
├── providers/     # 모델 어댑터
│   ├── openai.rs         # OpenAI API
│   ├── anthropic.rs      # Claude API
│   ├── gemini.rs         # Gemini API
│   └── ollama.rs         # Ollama 로컬
│
├── runtime/       # 런타임 서비스
│   ├── session.rs        # 세션 관리
│   ├── workflow_engine.rs # DAG 오케스트레이션
│   ├── mcp.rs            # MCP 서버
│   ├── cron/             # 작업 스케줄링
│   ├── terminal/         # 터미널 백엔드
│   ├── shell_hooks.rs    # Shell 통합
│   └── message_gateway/  # 플랫폼 통합
│
└── trajectory/   # 학습 시스템
    ├── memory.rs         # 메모리 관리
    ├── skill.rs          # 스킬 시스템
    ├── rl.rs             # RL 보상 신호
    ├── behavior_learner.rs # 패턴 학습
    └── user_profile.rs   # 사용자 프로파일링
```

### 프론트엔드 아키텍처

```
src/
├── stores/                    # Zustand 상태 관리
│   ├── domain/               # 코어 비즈니스 상태
│   │   ├── conversationStore.ts
│   │   ├── messageStore.ts
│   │   └── streamStore.ts
│   ├── feature/               # 기능 모듈 상태
│   │   ├── agentStore.ts
│   │   ├── gatewayStore.ts
│   │   ├── workflowEditorStore.ts
│   │   └── knowledgeStore.ts
│   └── shared/                # 공유 상태
│
├── components/
│   ├── chat/                # 채팅 인터페이스(60+ 컴포넌트)
│   ├── workflow/            # 워크플로 편집기
│   ├── gateway/             # API 게이트웨이 UI
│   ├── settings/            # 설정 패널
│   └── terminal/            # 터미널 UI
│
└── pages/                    # 페이지 컴포넌트
```

### 플랫폼 지원

| 플랫폼 | 아키텍처 |
|--------|----------|
| macOS | Apple Silicon (arm64), Intel (x86_64) |
| Windows | x86_64, ARM64 |
| Linux | x86_64, ARM64 (AppImage/deb/rpm) |

## 시작하기

### 사전 빌드 다운로드

[Releases](https://github.com/polite0803/AxAgent/releases) 페이지에서 플랫폼용 인스톨러를 다운로드하세요.

### 소스에서 빌드

#### 요구 사항

- [Node.js](https://nodejs.org/) 20+
- [Rust](https://www.rust-lang.org/) 1.75+
- [npm](https://www.npmjs.com/) 10+
- Windows: [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) + Rust MSVC targets

#### 빌드 단계

```bash
# 리포지토리 복제
git clone https://github.com/polite0803/AxAgent.git
cd AxAgent

# 종속성 설치
npm install

# 개발 모드
npm run tauri dev

# 프론트엔드만 빌드
npm run build

# 데스크톱 애플리케이션 빌드
npm run tauri build
```

빌드 아티팩트는 `src-tauri/target/release/`에 있습니다.

### 테스트

```bash
# 단위 테스트
npm run test

# E2E 테스트
npm run test:e2e

# 타입 확인
npm run typecheck
```

---

## 프로젝트 구조

```
AxAgent/
├── src/                         # 프론트엔드 소스 (React + TypeScript)
│   ├── components/              # React 컴포넌트
│   │   ├── chat/               # 채팅 인터페이스(60+ 컴포넌트)
│   │   ├── workflow/           # 워크플로 편집기 컴포넌트
│   │   ├── gateway/            # API 게이트웨이 컴포넌트
│   │   ├── settings/           # 설정 패널
│   │   └── terminal/          # 터미널 컴포넌트
│   ├── pages/                   # 페이지 컴포넌트
│   ├── stores/                  # Zustand 상태 관리
│   │   ├── domain/            # 코어 비즈니스 상태
│   │   └── feature/           # 기능 모듈 상태
│   ├── hooks/                   # React hooks
│   ├── lib/                     # 유틸리티 함수
│   ├── types/                   # TypeScript 타입 정의
│   └── i18n/                    # 12개 언어 번역
│
├── src-tauri/                    # 백엔드 소스 (Rust)
│   ├── crates/                  # Rust workspace(9개 crates)
│   │   ├── agent/             # AI 에이전트 코어
│   │   ├── core/              # 데이터베이스, 암호화, RAG
│   │   ├── gateway/           # API 게이트웨이 서버
│   │   ├── providers/         # 모델 프로바이더 어댑터
│   │   ├── runtime/           # 런타임 서비스
│   │   ├── trajectory/       # 메모리 및 학습
│   │   └── telemetry/        # 트레이싱 및 지표
│   └── src/                    # Tauri 진입점
│
├── e2e/                        # Playwright E2E 테스트
├── scripts/                    # 빌드 스크립트
└── docs/                       # 문서
```

## 데이터 디렉토리

```
~/.axagent/                      # 구성 디렉토리
├── axagent.db                   # SQLite 데이터베이스
├── master.key                   # AES-256 마스터 키
├── vector_db/                   # 벡터 데이터베이스 (sqlite-vec)
└── ssl/                         # SSL 인증서

~/Documents/axagent/            # 사용자 파일 디렉토리
├── images/                     # 이미지 첨부 파일
├── files/                      # 파일 첨부 파일
└── backups/                    # 백업 파일
```

---

## FAQ

### macOS: "앱이 손상되었습니다" 또는 "개발자를 확인할 수 없습니다"

앱이 Apple에서 서명하지 않았기 때문에:

**1. "모든 곳"의 앱 허용**
```bash
sudo spctl --master-disable
```

그런 다음 **시스템 설정 → 개인정보 보호 및 보안 → 보안**로 이동하여 **모든 곳**을 선택합니다.

**2. 검역 속성 제거**
```bash
sudo xattr -dr com.apple.quarantine /Applications/AxAgent.app
```

**3. macOS Ventura+ 추가 단계**
**시스템 설정 → 개인정보 보호 및 보안**로 이동하여 **그래도 열기**를 클릭합니다.

---

## 커뮤니티

- [LinuxDO](https://linux.do)

## 라이선스

이 프로젝트는 [AGPL-3.0](LICENSE) 라이선스 하에 라이선스됩니다.
