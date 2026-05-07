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
- **스트리밍 출력** — 실시간 토큰 단위 렌더링, 접이식 사고 블록(Claude 확장 사고) 지원
- **멀티 모델 비교** — 여러 모델에 동시에 동일한 질문을 전송하고 나란히 비교
- **함수 호출** — 지원되는 모든 프로바이더에 걸친 구조화된 함수 호출
- **OpenAI Responses API** — OpenAI Responses 형식 전송 지원
- **실시간 API** — OpenAI 실시간 API 호환 WebSocket 이벤트 푸시

### 🔐 AI 에이전트 시스템

에이전트 시스템은 정교한 아키텍처를 기반으로 구축되어 다음 기능을 제공합니다:

- **ReAct 추론 엔진** — 추론과 행동을 통합하고 자체 검증을 내장하여 작업 실행의 신뢰성 보장
- **계층적 플래너** — 복잡한 작업을 단계 및 의존성을 가진 구조화된 계획으로 분해
- **작업 분해기** — 복잡한 작업을 자동으로 실행 가능한 하위 작업으로 분해
- **심층 연구** — 다중 소스 검색 오케스트레이션, 인용 추적 및 신뢰도 평가
- **팩트체크** — AI 기반 사실 검증 및 출처 분류
- **검색 오케스트레이션** — 다중 검색 프로바이더 조정, 검색 계획 및 결과 종합 지원
- **학술 검색** — 학술 문헌 검색 및 인용 분석
- **컴퓨터 제어** — AI 제어 마우스 클릭, 키보드 입력, 화면 스크롤, 비전 모델 분석과 연계
- **화면 인식** — 스크린샷 캡처 및 비전 모델 분석으로 UI 요소 식별
- **3단계 권한 모드** — 기본(승인 필요), 편집 수락(자동 승인), 전체 액세스(프롬프트 없음)
- **샌드박스 격리** — 에이전트 작업은 지정된 작업 디렉토리로 엄격히 제한
- **도구 승인 패널** — 도구 호출 요청의 실시간 표시, 항목별 검토 지원
- **비용 추적** — 각 세션의 토큰 사용량 및 비용 통계 실시간 표시
- **일시 중지/재개** — 에이전트 실행을 언제든지 일시 중지하고 나중에 재개
- **체크포인트 시스템** — 크래시 복구 및 세션 재개를 위한 영속성 체크포인트
- **오류 복구 엔진** — 자동 오류 분류, 근본 원인 분석 및 복구 전략 실행
- **루프 감지** — 에이전트 추론에서 순환 동작 자동 감지 및 중단
- **사고 체인** — 에이전트 결정 추론의 시각화, 단계별 분해
- **능동적 모드** — 에이전트가 자발적으로 제안 및 작업 실행 가능
- **목적 관리** — 에이전트의 실행 목적 및 컨텍스트 유지 및 추적

### 👥 멀티 에이전트 협업

- **하위 에이전트 조정** — 마스터-슬레이브 아키텍처로 여러 협업 에이전트 지원
- **병렬 실행** — 여러 에이전트가 작업을 병렬 처리, 의존성 인식 스케줄링 지원
- **적대적 디베이트** — Pro/Con 디베이트 라운드, 논점 강도 점수 매기기 및 반박 추적 지원
- **에이전트 역할** — 팀 협업을 위한 사전 정의된 역할(연구자, 플래너, 개발자, 검토자, 종합자)
- **에이전트 오케스트레이터** — 멀티 에이전트 팀을 위한 중앙 집중식 메시지 라우팅 및 상태 관리
- **통신 그래프** — 에이전트 상호작용 및 메시지 흐름의 시각적 표현
- **Swarm 클러스터** — 다중 프로세스 에이전트 클러스터, 권한 동기화 및 자동 재연결 지원
- **Buddy 파트너 시스템** — 구성 가능한 에이전트 파트너, 종족 및 속성 정의 지원
- **공유 메모리** — 에이전트 간 공유 메모리 공간, 통계 및 쿼리 지원
- **팀 Cron 등록** — 팀 수준의 정기 작업 스케줄링

### ⭐ 스킬 시스템

- **스킬 마켓플레이스** — 커뮤니티 기여 스킬을 검색하고 설치할 수 있는 내장 마켓플레이스
- **스킬 생성** — 제안에서 자동으로 스킬 생성, Markdown 편집기 지원
- **스킬 진화** — 실행 피드백에 기반한 AI 구동 기존 스킬의 자동 분석 및 개선
- **스킬 매칭** — 의미적 매칭으로 대화 컨텍스트와 관련된 스킬 추천
- **스킬 분해** — 복잡한 작업을 자동으로 실행 가능한 원자 스킬로 분해(LLM 보조/다중 라운드/워크플로 검증)
- **생성 도구** — AI가 자동으로 새로운 도구를 생성하고 등록하여 에이전트 능력 확장
- **스킬 허브** — 중앙 집중식 스킬 발견 및 구성 관리 인터페이스
- **스킬 허브 클라이언트** — 원격 스킬 허브와의 통합, 커뮤니티 공유 지원
- **스킬 의존성 검사** — 스킬 의존성 및 도구 가용성 자동 검사
- **스킬 샌드박스 컨테이너** — 격리된 환경에서 스킬 안전 실행

### 🔄 워크플로 시스템

워크플로 엔진은 DAG 기반 작업 오케스트레이션 시스템을 구현합니다:

- **시각적 워크플로 편집기** — 노드 연결 및 구성을 지원하는 드래그 앤 드롭 워크플로 디자이너
- **풍부한 노드 유형** — 15가지 노드 유형: 트리거, 에이전트, LLM, 조건, 병렬, 루프, 병합, 지연, 도구, 코드, 하위 워크플로, 벡터 검색, 문서 파서, 검증, 종료
- **워크플로 템플릿** — 내장 프리셋: 코드 리뷰, 버그 수정, 문서, 테스트, 리팩토링, 탐색, 성능, 보안, 기능 개발
- **DAG 실행** — Kahn 알고리즘 위상 정렬, 순환 감지 지원
- **병렬 디스패치** — 파이프라인 스타일 실행, 빠른 단계가 느린 단계를 기다리지 않음
- **재시도 정책** — 지수 백오프, 각 단계별 구성 가능한 최대 재시도 횟수
- **부분 완료** — 실패한 단계가 독립적인 하류 단계를 차단하지 않음
- **버전 관리** — 워크플로 템플릿 버전 관리, 롤백 지원
- **실행 기록** — 상세한 기록, 상태 추적 및 디버깅 지원
- **AI 지원** — AI 지원 워크플로 설계, 노드 추천 및 에이전트 프롬프트 최적화
- **의미 검사** — 워크플로 의미 검증, 잠재적 문제 감지
- **n8n 가져오기** — n8n 디렉토리에서 워크플로 가져오기 지원
- **디버그 패널** — 워크플로 실행 과정의 실시간 디버깅 및 상태 확인

### 📚 지식 및 메모리

- **지식 베이스(RAG)** — 멀티 지식 베이스 지원, 문서 업로드, 자동 분석, 청킹 및 벡터 인덱싱 지원
- **하이브리드 검색** — 벡터 유사성 검색과 BM25 전체 텍스트 순위 조합
- **리랭킹** — 교차 인코더 리랭킹으로 검색 정확도 향상
- **3단계 리콜 파이프라인** — AST 인덱스 + 벡터 검색 + FTS5의 다단계 리콜 메커니즘
- **지식 그래프** — 지식 연결의 엔티티 관계 시각화(엔티티, 속성, 관계, 흐름, 인터페이스)
- **Wiki 시스템** — LLM Wiki 컴파일러 및 검증기, 지식 그래프 시각화 및 증분 동기화 지원
- **Wiki 노트** — 양방향 링크 노트 시스템, 그래프 뷰 및 자동 링크 동기화 지원
- **메모리 시스템** — 멀티 네임스페이스 메모리, 수동 입력 또는 AI 자동 추출 지원
- **폐쇄 루프 메모리** — Honcho 및 Mem0 영속성 메모리 프로바이더와의 통합
- **FTS5 전체 텍스트 검색** — 대화, 파일, 메모리 전체의 빠른 검색
- **세션 검색** — 모든 대화 세션 전체의 고급 검색
- **컨텍스트 관리** — 파일, 검색 결과, 지식 스니펫, 메모리, 도구 출력의 유연한 첨부
- **문서 파싱** — 다중 형식 문서 자동 파싱 및 콘텐츠 추출
- **증분 인덱싱** — 파일 변경에 대한 증분 인덱스 업데이트

### 🌐 API 게이트웨이

- **로컬 API 서버** — 내장 OpenAI 호환, Claude 및 Gemini 인터페이스 서버
- **외부 링크** — 원클릭 Claude CLI, OpenCode 통합, API 키 및 모델 자동 동기화
- **키 관리** — 생성, 취소, 활성화/비활성화, 설명이 있는 액세스 키 관리
- **사용량 분석** — 키, 프로바이더, 날짜별 요청량 및 토큰 사용량
- **SSL/TLS 지원** — 내장 자체 서명 인증서, 사용자 정의 인증서 지원
- **요청 로깅** — 모든 API 요청 및 응답의 완전한 기록
- **구성 템플릿** — Claude, Codex, OpenCode, Gemini의 사전 구축된 템플릿
- **실시간 API** — OpenAI 실시간 API 호환 WebSocket 이벤트 푸시
- **플랫폼 통합** — DingTalk, Feishu, QQ, Slack, WeChat, WhatsApp, Telegram, Discord 지원
- **게이트웨이 진단** — 연결 진단 및 프로그램 정책 관리
- **속도 제한기** — API 요청 속도 제한 및 트래픽 제어
- **영속성 큐** — 요청 영속성 큐 관리

### 🔧 도구 및 확장

- **MCP 프로토콜** — 완전한 모델 컨텍스트 프로토콜 구현, stdio 및 HTTP/WebSocket 전송 지원
- **OAuth 인증** — MCP 서버의 OAuth 흐름 지원
- **MCP 자동 시작** — MCP 서버 자동 시작 및 수명 주기 관리
- **MCP 도구 브릿지** — MCP 도구와 에이전트 도구 시스템의 브릿지
- **플러그인 시스템** — 내장/번들/외부 3단계 플러그인 아키텍처, 도구 등록, 훅 및 수명 주기 관리 지원
- **내장 도구** — 종합적인 파일 작업(읽기/쓰기/편집), 코드 실행, 검색(Grep/Glob), Bash, 웹 검색, 웹 스크래핑, 계획 관리, Cron 스케줄링, REPL, LSP, 컨텍스트 관리, 컴퓨터 제어, 메시지 푸시, 할 일 등
- **도구 권한 시스템** — 도구 권한 분류, 규칙 관리 및 사용 추적
- **Bash 보안** — 명령 파싱, 경로 검증 및 샌드박스 보안 제어
- **LSP 클라이언트** — 내장 언어 서버 프로토콜, 코드 완성 및 진단 지원
- **AST 인덱스** — 코드 파일의 AST 파싱 및 인덱스 구축
- **터미널 백엔드** — 로컬, Docker 및 SSH 터미널 연결 지원
- **브라우저 자동화** — CDP를 통한 브라우저 제어 기능 통합(탐색, 스크린샷, 클릭, 폼 작성, 텍스트 추출 등)
- **UI 자동화** — 크로스 플랫폼 UI 요소 식별 및 제어
- **Git 도구** — 분기 감지 및 충돌 인식을 지원하는 Git 작업
- **도구 추천** — 컨텍스트 기반 스마트 도구 추천 엔진
- **도구 오케스트레이션** — 다중 도구 조정 실행 및 스트리밍 출력
- **도구 통계** — 도구 사용 빈도 및 성능 통계

### 📊 콘텐츠 렌더링

- **Markdown 렌더링** — 코드 하이라이트, LaTeX 수학, 표, 작업 목록의 완전한 지원
- **Monaco 코드 편집기** — 내장 편집기, 구문 하이라이트, 복사, 차이점 미리보기 지원
- **다이어그램 렌더링** — Mermaid 플로우차트, D2 아키텍처 다이어그램, ECharts 대화형 차트
- **아티팩트 패널** — 코드 스니펫, HTML 초안, React 구성 요소, Markdown 노트, 실시간 미리보기 지원
- **4가지 미리보기 모드** — 코드(편집기), 분할(나란히), 미리보기(렌더링만), React 구성 요소 미리보기
- **세션 검사기** — 세션 구조의 트리 뷰, 빠른 탐색
- **인용 패널** — 소스 인용 추적 및 표시, 신뢰도 점수 매기기 지원
- **인포그래픽 렌더링** — 인포그래픽 시각화 표시 지원

### 🛡️ 데이터 및 보안

- **AES-256 암호화** — API 키 및 민감한 데이터는 AES-256-GCM으로 암호화
- **분리 저장소** — 애플리케이션 상태는 `~/.axagent/`에, 사용자 파일은 `~/Documents/axagent/`에 저장
- **자동 백업** — 로컬 디렉토리 또는 WebDAV 저장소로 예약된 백업
- **백업 복원** — 원클릭으로 이전 백업에서 복원
- **내보내기 옵션** — PNG 스크린샷, Markdown, 일반 텍스트, JSON 형식
- **저장소 관리** — 시각적 디스크 사용량 표시 및 정리 도구
- **파일 권한 부여** — 파일 액세스 권한 부여 및 취소 관리
- **작업 감사** — 주요 작업의 감사 로그 기록

### 🖥️ 데스크톱 환경

- **테마 엔진** — 다크/라이트 테마, 시스템 따르기 또는 수동 기본 설정 지원
- **인터페이스 언어** — 11개 언어: 간체 중국어, 번체 중국어, 영어, 일본어, 한국어, 프랑스어, 독일어, 스페인어, 러시아어, 힌디어, 아랍어
- **시스템 트레이** — 백그라운드 서비스를 중단하지 않고 트레이로 최소화
- **항상 위에** — 다른 창보다 앞에 창 고정
- **전역 단축키** — 주 창을 호출하기 위한 사용자 정의 가능한 단축키
- **QuickBar** — 빠른 액세스 플로팅 바, 원클릭 호출
- **자동 시작** — 시스템 시작 시 선택적 실행
- **프록시 지원** — HTTP 및 SOCKS5 프록시 구성
- **자동 업데이트** — 자동 버전 확인 및 업데이트 프롬프트
- **명령 팔레트** — `Cmd/Ctrl+K` 빠른 명령 액세스
- **온보딩 마법사** — 최초 사용 시 대화형 가이드 및 Ollama 감지
- **알림 센터** — 통합 앱 내 알림 관리

### 🔬 고급 기능

- **심층 연구** — 다중 소스 검색, 인용 추적, 신뢰도 평가 및 콘텐츠 종합
- **팩트체크** — AI 기반 사실 검증 및 출처 분류
- **Cron 스케줄러** — 매일/매주/매월 템플릿 및 사용자 정의 cron 표현식을 통한 자동화된 작업 스케줄링
- **Webhook 시스템** — 도구 완료, 에이전트 오류, 세션 종료 알림의 이벤트 구독
- **사용자 프로파일링** — 코드 스타일, 명명 규칙, 들여쓰기, 주석 스타일, 커뮤니케이션 기본 설정의 자동 학습
- **RL 옵티마이저** — 도구 선택 및 작업 전략 최적화를 위한 강화 학습
- **LoRA 미세 조정** — LoRA를 사용한 로컬 교육으로 사용자 정의 모델 어댑테이션
- **능동적 제안** — 대화 내용 및 사용자 패턴에 기반한 컨텍스트 인식 힌트
- **컨텍스트 예측** — 사용자의 다음 작업을 예측하고 관련 리소스 사전 로드
- **드림 통합** — 백그라운드에서 메모리와 패턴을 자동 통합하여 장기 지식 최적화
- **오류 복구** — 자동 오류 분류, 근본 원인 분석 및 복구 제안
- **개발자 도구** — 디버깅 및 성능 분석을 위한 Trace, Span, 타임라인 시각화
- **벤치마크 시스템** — SWE-bench / Terminal-bench 작업 성능 평가 및 지표, 점수 카드 포함
- **스타일 전송** — 학습한 코드 스타일 기본 설정을 생성된 코드에 적용
- **대시보드 플러그인** — 사용자 정의 패널 및 위젯을 지원하는 확장 가능한 대시보드
- **협업 공유** — CRDT 실시간 협업 및 원클릭 세션 공유
- **브라우저 확장** — Wiki Clipper 브라우저 확장, 웹 페이지를 LLM Wiki로 빠르게 클리핑
- **Python SDK** — AxAgent와의 통합을 위한 Python SDK 제공
- **스마트 라우팅** — 요청 스마트 라우팅 및 분류
- **의미 캐시** — 의미 기반 응답 캐시, 중복 계산 감소
- **컨텍스트 압축** — 긴 컨텍스트 자동 압축, 토큰 사용 최적화
- **메시지 배치 처리** — 메시지 배치 전송 및 최적화
- **연결 풀** — 데이터베이스 및 API 연결 풀 관리
- **기능 플래그** — 구성 가능한 기능 특성 토글 시스템
- **정책 엔진** — 권한 및 작업 정책의 중앙 집중식 관리
- **리소스 거버넌스** — 에이전트 리소스 사용 제한 및 거버넌스
- **LAN 전송** — 로컬 영역 네트워크 파일 전송 기능

---

## 기술 아키텍처

### 기술 스택

| 레이어 | 기술 |
|--------|------|
| **프레임워크** | Tauri 2 + React 19 + TypeScript 6 |
| **UI** | Ant Design 6 + TailwindCSS 4 |
| **상태 관리** | Zustand 5 |
| **라우팅** | React Router 7 |
| **i18n** | i18next + react-i18next |
| **백엔드** | Rust + SeaORM 2 + SQLite |
| **벡터 DB** | sqlite-vec |
| **코드 편집기** | Monaco Editor |
| **다이어그램** | Mermaid + D2 + ECharts(CDN) |
| **터미널** | xterm.js 6 |
| **워크플로** | ReactFlow 11 |
| **빌드** | Vite 8 + npm |

### Rust 백엔드 아키텍처

백엔드는 전문화된 10개의 crates로 구성된 Rust workspace로 구성됩니다:

```
src-tauri/crates/
├── agent/         # AI 에이전트 코어
│   ├── react_engine.rs          # ReAct 추론 엔진
│   ├── coordinator.rs           # 에이전트 조정
│   ├── hierarchical_planner.rs  # 작업 분해
│   ├── task_decomposer.rs       # 하위 작업 분해
│   ├── self_verifier.rs         # 출력 검증
│   ├── verification_agent.rs    # 검증 에이전트
│   ├── error_recovery_engine.rs # 오류 복구 엔진
│   ├── error_classifier.rs      # 오류 분류
│   ├── recovery_strategies.rs   # 복구 전략
│   ├── loop_detector.rs         # 루프 감지
│   ├── vision_pipeline.rs       # 화면 인식
│   ├── deep_research.rs         # 심층 연구
│   ├── fact_checker.rs          # 팩트체크
│   ├── research_agent.rs        # 연구 에이전트
│   ├── search_planner.rs        # 검색 계획
│   ├── search_orchestrator.rs   # 검색 오케스트레이션
│   ├── academic_search.rs       # 학술 검색
│   ├── source_validator.rs      # 출처 검증
│   ├── source_classifier.rs     # 출처 분류
│   ├── credibility_evaluator.rs # 신뢰도 평가
│   ├── citation_tracker.rs      # 인용 추적
│   ├── content_synthesizer.rs   # 콘텐츠 종합
│   ├── outline_builder.rs       # 개요 구축
│   ├── reference_builder.rs     # 참조 구축
│   ├── proactive_mode.rs        # 능동적 모드
│   ├── purpose_manager.rs       # 목적 관리
│   ├── graph_insights.rs        # 그래프 인사이트
│   ├── insight_generator.rs     # 인사이트 생성
│   ├── schema_manager.rs        # Schema 관리
│   ├── ingest_pipeline.rs       # 데이터 수집 파이프라인
│   ├── session_manager.rs       # 세션 관리
│   ├── health_checker.rs        # 상태 확인
│   ├── metrics.rs               # 지표 수집
│   ├── evaluator/               # 벤치마크 평가
│   ├── fine_tune/               # LoRA 미세 조정
│   ├── rl_optimizer/            # RL 정책 최적화
│   └── tool_recommender/        # 도구 추천 엔진
│
├── core/          # 코어 유틸리티
│   ├── db.rs                   # SeaORM 데이터베이스
│   ├── vector_store.rs         # sqlite-vec 통합
│   ├── rag.rs                  # RAG 추상화 레이어
│   ├── hybrid_search.rs        # 벡터 + FTS5 검색
│   ├── recall_pipeline.rs      # 3단계 리콜 파이프라인
│   ├── crypto.rs               # AES-256 암호화
│   ├── mcp_client.rs           # MCP 프로토콜 클라이언트
│   ├── browser_automation.rs   # 브라우저 자동화
│   ├── computer_control.rs     # 컴퓨터 제어
│   ├── screen_vision.rs        # 화면 비전
│   ├── screen_capture.rs       # 화면 캡처
│   ├── ui_automation.rs        # UI 자동화
│   ├── ast_index.rs            # AST 인덱스
│   ├── incremental_indexer.rs  # 증분 인덱서
│   ├── document_parser.rs      # 문서 파싱
│   ├── markdown_parser.rs      # Markdown 파싱
│   ├── text_chunker.rs         # 텍스트 청킹
│   ├── token_counter.rs        # 토큰 카운터
│   ├── token_budget.rs         # 토큰 예산
│   ├── file_index.rs           # 파일 인덱스
│   ├── file_authorizer.rs      # 파일 권한 부여
│   ├── file_store.rs           # 파일 저장소
│   ├── cache.rs                # 캐시 관리
│   ├── disk_cache.rs           # 디스크 캐시
│   ├── cache_persister.rs      # 캐시 영속화
│   ├── cache_snapshot.rs       # 캐시 스냅샷
│   ├── vector_cache.rs         # 벡터 캐시
│   ├── marketplace_service.rs  # 마켓플레이스 서비스
│   ├── marketplace.rs          # 마켓플레이스 추상화
│   ├── operation_audit.rs      # 작업 감사
│   ├── unified_config.rs       # 통합 구성
│   ├── platform_config.rs      # 플랫폼 구성
│   ├── command_validator.rs    # 명령 검증
│   ├── shell_parser.rs         # Shell 파싱
│   ├── output_processor.rs     # 출력 처리
│   ├── storage_inventory.rs    # 저장소 인벤토리
│   ├── storage_migration.rs    # 저장소 마이그레이션
│   ├── storage_paths.rs        # 저장소 경로
│   ├── s3_backup.rs            # S3 백업
│   ├── webdav.rs               # WebDAV 동기화
│   ├── git_tools.rs            # Git 도구
│   ├── sandbox_runner.rs       # 샌드박스 러너
│   ├── search.rs               # 검색 추상화
│   ├── reranker.rs             # 리랭커
│   ├── model_knowledge.rs      # 모델 지식
│   ├── prompt_template.rs      # 프롬프트 템플릿
│   ├── preset_templates.rs     # 프리셋 템플릿
│   ├── workflow_types.rs       # 워크플로 타입
│   ├── workflow_version.rs     # 워크플로 버전
│   ├── path_vars.rs            # 경로 변수
│   ├── entity/                 # SeaORM 엔티티(40+ 테이블)
│   └── repo/                   # 데이터 리포지토리(30+ 리포지토리)
│
├── gateway/       # API 게이트웨이
│   ├── server.rs               # HTTP 서버
│   ├── handlers.rs             # API 핸들러
│   ├── routes.rs               # 라우트 정의
│   ├── auth.rs                 # 인증
│   ├── middleware.rs           # 미들웨어
│   ├── metrics.rs              # 지표 수집
│   ├── native.rs               # 네이티브 통합
│   ├── marketplace_handlers.rs # 마켓플레이스 인터페이스
│   └── realtime.rs             # WebSocket 지원
│
├── plugins/       # 플러그인 시스템
│   ├── hooks.rs                # 훅 러너
│   ├── agent_provider.rs       # 에이전트 프로바이더
│   ├── test_isolation.rs       # 테스트 격리
│   └── lib.rs                  # 플러그인 레지스트리 및 수명 주기
│
├── providers/     # 모델 어댑터
│   ├── adapter.rs              # 어댑터 인터페이스
│   ├── registry.rs             # 프로바이더 레지스트리
│   ├── openai.rs               # OpenAI API
│   ├── openai_responses.rs     # OpenAI Responses API
│   ├── anthropic.rs            # Claude API
│   ├── gemini.rs               # Gemini API
│   ├── ollama.rs               # Ollama 로컬
│   ├── openclaw.rs             # OpenClaw
│   ├── hermes.rs               # Hermes
│   ├── image_gen.rs            # 이미지 생성
│   ├── realtime_client.rs      # 실시간 API 클라이언트
│   └── transport/              # 전송 계층(Chat Completions / Responses / Anthropic)
│
├── runtime/       # 런타임 서비스
│   ├── session.rs              # 세션 관리
│   ├── workflow_engine.rs      # DAG 오케스트레이션
│   ├── work_engine/            # 워크 엔진(노드 실행기 + 스케줄러 + 캐시 레이어)
│   ├── mcp.rs                  # MCP 서버
│   ├── mcp_client.rs           # MCP 클라이언트
│   ├── mcp_server.rs           # MCP 서버 구현
│   ├── mcp_stdio.rs            # MCP stdio 전송
│   ├── mcp_autostart.rs        # MCP 자동 시작
│   ├── mcp_lifecycle_hardened.rs # MCP 수명 주기 관리
│   ├── mcp_tool_bridge.rs      # MCP 도구 브릿지
│   ├── cron/                   # 작업 스케줄링
│   ├── terminal/               # 터미널 백엔드(로컬/Docker/SSH)
│   ├── benchmarks/             # SWE-bench / Terminal-bench
│   ├── collaboration/          # CRDT 협업 및 세션 공유
│   ├── tool_generator/         # AI 도구 생성
│   ├── message_gateway/        # 플랫폼 통합(DingTalk/Feishu/QQ/Slack/WeChat/WhatsApp/Telegram/Discord)
│   ├── buddy/                  # Buddy 파트너 시스템(종족/속성/관리자)
│   ├── swarm/                  # Swarm 클러스터(프로세스 백엔드/권한 동기화/재연결)
│   ├── tasks/                  # 백그라운드 작업(드림/원격 에이전트/인프로세스 팀원)
│   ├── adversarial_debate.rs   # 적대적 디베이트
│   ├── agent_orchestrator.rs   # 멀티 에이전트 오케스트레이션
│   ├── agent_roles.rs          # 에이전트 역할
│   ├── webhook_dispatcher.rs   # Webhook 디스패처
│   ├── webhook_server.rs       # Webhook 서버
│   ├── session_search.rs       # 세션 검색
│   ├── dashboard_plugin.rs     # 대시보드 플러그인
│   ├── dashboard_registry.rs   # 대시보드 레지스트리
│   ├── permissions.rs          # 권한 관리
│   ├── permission_enforcer.rs  # 권한 실행
│   ├── policy_engine.rs        # 정책 엔진
│   ├── trust_resolver.rs       # 신뢰 해석
│   ├── resource_governor.rs    # 리소스 거버넌스
│   ├── green_contract.rs       # 그린 컨트랙트
│   ├── feature_flags.rs        # 기능 플래그
│   ├── module_switch.rs        # 모듈 스위치
│   ├── mode_selector.rs        # 모드 선택
│   ├── config.rs               # 런타임 구성
│   ├── config_validate.rs      # 구성 검증
│   ├── prompt.rs               # 프롬프트 관리
│   ├── prompt_cache.rs         # 프롬프트 캐시
│   ├── compact.rs              # 컨텍스트 압축
│   ├── summary_compression.rs  # 요약 압축
│   ├── compact_thresholds.rs   # 압축 임계값
│   ├── compact_warning.rs      # 압축 경고
│   ├── reactive_compact.rs     # 반응형 압축
│   ├── session_memory_compact.rs # 세션 메모리 압축
│   ├── message_importance.rs   # 메시지 중요도 평가
│   ├── message_batching.rs     # 메시지 배치 처리
│   ├── rate_limiter.rs         # 속도 제한기
│   ├── connection_pool.rs      # 연결 풀
│   ├── persistent_queue.rs     # 영속성 큐
│   ├── persistent_queue_manager.rs # 큐 관리자
│   ├── health_check.rs         # 상태 확인
│   ├── cache_guard.rs          # 캐시 가드
│   ├── checkpoint.rs           # 체크포인트
│   ├── branch_lock.rs          # 브랜치 잠금
│   ├── stale_base.rs           # 만료 기준선 감지
│   ├── watch_patterns.rs       # 감시 패턴
│   ├── lan_transfer.rs         # LAN 전송
│   ├── tls_config.rs           # TLS 구성
│   ├── sse.rs                  # SSE 이벤트 스트림
│   ├── api_server.rs           # API 서버
│   ├── gateway_auth.rs         # 게이트웨이 인증
│   ├── gateway_metrics.rs      # 게이트웨이 지표
│   ├── bash.rs                 # Bash 실행
│   ├── bash_validation.rs      # Bash 검증
│   ├── shell_hooks.rs          # Shell 훅
│   ├── shell_completer.rs      # Shell 자동완성
│   ├── terminal_analyzer.rs    # 터미널 분석
│   ├── git_context.rs          # Git 컨텍스트
│   ├── git_tools.rs            # Git 도구
│   ├── file_ops.rs             # 파일 작업
│   ├── hooks.rs                # 훅 관리
│   ├── hook_chain.rs           # 훅 체인
│   ├── hook_config.rs          # 훅 구성
│   ├── plugin_hooks.rs         # 플러그인 훅
│   ├── plugin_lifecycle.rs     # 플러그인 수명 주기
│   ├── profile.rs              # 프로필
│   ├── profile_manager.rs      # 프로필 관리자
│   ├── oauth.rs                # OAuth 인증
│   ├── usage.rs                # 사용량 통계
│   ├── bootstrap.rs            # 부트스트랩
│   ├── worker_boot.rs          # Worker 부트
│   ├── fork_bridge.rs          # Fork 브릿지
│   ├── task_packet.rs          # 작업 패킷
│   ├── task_router.rs          # 작업 라우터
│   ├── task_registry.rs        # 작업 레지스트리
│   ├── transform_pipeline.rs   # 변환 파이프라인
│   ├── transport_handlers.rs   # 전송 핸들러
│   ├── general_engine.rs       # 범용 엔진
│   ├── engine_bridge.rs        # 엔진 브릿지
│   ├── conversation.rs         # 대화 관리
│   ├── session_control.rs      # 세션 제어
│   ├── shared_memory.rs        # 공유 메모리
│   ├── validation_executor.rs  # 검증 실행기
│   ├── recovery_recipes.rs     # 복구 레시피
│   ├── error_recovery.rs       # 오류 복구
│   ├── theme_engine.rs         # 테마 엔진
│   ├── token_budget_predictor.rs # 토큰 예산 예측
│   ├── team_cron_registry.rs   # 팀 Cron 등록
│   ├── module_dream.rs         # 드림 모듈
│   ├── json.rs                 # JSON 도구
│   └── lane_events.rs          # Lane 이벤트
│
├── telemetry/     # 원격 측정 및 추적
│   ├── tracer.rs              # 분산 추적
│   ├── metrics.rs             # 지표 수집
│   ├── span.rs                # Span 관리
│   ├── event.rs               # 이벤트 정의
│   ├── collector.rs           # 데이터 수집
│   ├── exporter.rs            # 데이터 내보내기
│   └── storage.rs             # 저장소 백엔드
│
├── tools/         # 도구 시스템
│   ├── registry.rs             # 도구 레지스트리
│   ├── builtin_tools.rs        # 내장 도구 정의
│   ├── builtin_handlers.rs     # 내장 도구 핸들러
│   ├── orchestration.rs        # 도구 오케스트레이션
│   ├── streaming.rs            # 스트리밍 출력
│   ├── stats.rs                # 사용 통계
│   ├── recorder.rs             # 실행 기록
│   ├── agent_def_loader.rs     # 에이전트 정의 로더
│   ├── agent_def_types.rs      # 에이전트 정의 타입
│   ├── bash/                   # Bash 도구(파서/샌드박스/보안/경로 검증)
│   ├── hooks/                  # 훅(레지스트리/실행기)
│   ├── mcp/                    # MCP 도구(레지스트리/OAuth/래퍼)
│   ├── permissions/            # 권한(분류기/규칙/추적기)
│   └── tools/                  # 구체적 도구 구현
│       ├── agent.rs            # 에이전트 도구
│       ├── bash.rs             # Bash 실행
│       ├── context.rs          # 컨텍스트 관리
│       ├── cron.rs             # Cron 스케줄링
│       ├── glob.rs             # 파일 글로브
│       ├── grep.rs             # 콘텐츠 검색
│       ├── lsp.rs              # LSP 도구
│       ├── monitor.rs          # 모니터 도구
│       ├── plan.rs             # 계획 도구
│       ├── repl.rs             # REPL 도구
│       ├── skill.rs            # 스킬 도구
│       ├── web_fetch.rs        # 웹 스크래핑
│       ├── web_search.rs       # 웹 검색
│       ├── file_read.rs        # 파일 읽기
│       ├── file_write.rs       # 파일 쓰기
│       ├── file_edit.rs        # 파일 편집
│       ├── computer_use.rs     # 컴퓨터 제어
│       ├── messaging.rs        # 메시지 전송
│       ├── push_notification.rs # 푸시 알림
│       ├── task_system.rs      # 작업 시스템
│       ├── todo_write.rs       # 할 일 작성
│       └── batch_missing.rs    # 배치 누락 감지
│
├── trajectory/    # 학습 시스템
│   ├── memory.rs              # 메모리 관리
│   ├── memory_provider.rs     # 메모리 프로바이더 인터페이스
│   ├── auto_memory.rs         # 자동 메모리 추출
│   ├── skill.rs               # 스킬 시스템
│   ├── skill_manager.rs       # 스킬 관리자
│   ├── skill_evolution.rs     # 스킬 진화
│   ├── skill_matcher.rs       # 스킬 매칭
│   ├── skill_proposal.rs      # 스킬 제안
│   ├── skills_hub_adapter.rs  # 스킬 허브 어댑터
│   ├── skills_hub_client.rs   # 스킬 허브 클라이언트
│   ├── skill_decomposition/   # 스킬 분해(LLM 보조/다중 라운드/워크플로 검증/도구 파싱)
│   ├── rl.rs                  # RL 보상 신호
│   ├── rl_trainer.rs          # RL 트레이너
│   ├── training_env.rs        # 훈련 환경
│   ├── behavior_learner.rs    # 행동 학습
│   ├── behavior_tracker.rs    # 행동 추적
│   ├── pattern.rs             # 패턴 인식
│   ├── pattern_analyzer.rs    # 패턴 분석
│   ├── user_profile.rs        # 사용자 프로파일링
│   ├── preference_learner.rs  # 선호도 학습
│   ├── adaptation.rs          # 적응형 조정
│   ├── dream_consolidation.rs # 드림 통합
│   ├── parallel_execution.rs  # 병렬 실행 서비스
│   ├── style_extractor.rs     # 스타일 추출
│   ├── style_applier.rs       # 스타일 적용
│   ├── style_vectorizer.rs    # 스타일 벡터화
│   ├── style_migrator.rs      # 스타일 전송
│   ├── suggestion_engine.rs   # 제안 엔진
│   ├── proactive_assistant.rs # 능동적 어시스턴트
│   ├── context_predictor.rs   # 컨텍스트 예측
│   ├── task_prefetcher.rs     # 작업 사전 로드
│   ├── reminder_manager.rs    # 리마인더 관리
│   ├── nudge.rs               # 넛지 시스템
│   ├── insight.rs             # 인사이트 생성
│   ├── compactor.rs           # 데이터 압축
│   ├── trajectory.rs          # 궤적 관리
│   ├── trajectory_compressor.rs # 궤적 압축
│   ├── sub_agent.rs           # 하위 에이전트
│   ├── batch.rs               # 배치 처리
│   ├── context.rs             # 컨텍스트 관리
│   ├── fts5.rs                # FTS5 검색
│   ├── hooks.rs               # 훅
│   ├── storage.rs             # 저장소
│   ├── scheduled_task.rs      # 예약 작업
│   └── memory_providers/      # 메모리 프로바이더(Honcho/Mem0/폐쇄 루프/서비스)
│
└── migration/     # 데이터베이스 마이그레이션
    └── m20240101_000001~000010  # 10개 마이그레이션 파일
```

### 프론트엔드 아키텍처

```
src/
├── stores/                    # Zustand 상태 관리
│   ├── domain/               # 코어 비즈니스 상태
│   │   ├── conversationStore.ts
│   │   ├── messageStore.ts
│   │   ├── streamStore.ts
│   │   ├── multiModelStore.ts
│   │   ├── preferenceStore.ts
│   │   └── compressStore.ts
│   ├── feature/               # 기능 모듈 상태(30+ store)
│   │   ├── agentStore.ts
│   │   ├── agentProfileStore.ts
│   │   ├── appConfigStore.ts
│   │   ├── backupStore.ts
│   │   ├── buddyStore.ts
│   │   ├── categoryStore.ts
│   │   ├── decompositionStore.ts
│   │   ├── dreamStore.ts
│   │   ├── executionStore.ts
│   │   ├── expertStore.ts
│   │   ├── fileStore.ts
│   │   ├── gatewayStore.ts
│   │   ├── gatewayLinkStore.ts
│   │   ├── generatedToolStore.ts
│   │   ├── helpStore.ts
│   │   ├── knowledgeStore.ts
│   │   ├── llmWikiStore.ts
│   │   ├── localToolStore.ts
│   │   ├── memoryStore.ts
│   │   ├── mcpStore.ts
│   │   ├── nudgeStore.ts
│   │   ├── onboardingStore.ts
│   │   ├── planStore.ts
│   │   ├── platformStore.ts
│   │   ├── proactiveStore.ts
│   │   ├── promptTemplateStore.ts
│   │   ├── providerStore.ts
│   │   ├── searchStore.ts
│   │   ├── settingsStore.ts
│   │   ├── skillExtensionStore.ts
│   │   ├── skillStore.ts
│   │   ├── styleStore.ts
│   │   ├── terminalStore.ts
│   │   ├── themeStore.ts
│   │   ├── trajectoryStore.ts
│   │   ├── userProfileStore.ts
│   │   ├── wikiStore.ts
│   │   ├── workEngineStore.ts
│   │   └── workflowEditorStore.ts
│   ├── devtools/              # 개발자 도구 상태
│   │   ├── tracerStore.ts
│   │   ├── evaluatorStore.ts
│   │   ├── rlStore.ts
│   │   ├── fineTuneStore.ts
│   │   └── recommendationStore.ts
│   └── shared/                # 공유 상태
│       ├── artifactStore.ts
│       ├── chatWorkspaceStore.ts
│       ├── tabStore.ts
│       └── uiStore.ts
│
├── components/                # React 컴포넌트(24개 모듈)
│   ├── chat/                # 채팅 인터페이스(90+ 컴포넌트)
│   ├── workflow/            # 워크플로 편집기(노드/패널/템플릿/AI 보조)
│   ├── gateway/             # API 게이트웨이 UI
│   ├── settings/            # 설정 패널(40+ 컴포넌트)
│   ├── terminal/            # 터미널 UI
│   ├── skill/               # 스킬 편집기 및 렌더러
│   ├── benchmark/           # 벤치마크 패널
│   ├── decomposition/       # 스킬 분해 및 도구 생성
│   ├── files/               # 파일 관리 페이지
│   ├── fine-tune/           # LoRA 미세 조정 구성
│   ├── link/                # 외부 링크 관리
│   ├── llm-wiki/            # LLM Wiki 편집기
│   ├── proactive/           # 능동적 제안 시스템
│   ├── recommendation/      # 도구 추천 패널
│   ├── wiki/                # Wiki 관리
│   ├── devtools/            # Trace/Span 타임라인
│   ├── style/               # 코드 스타일 전송
│   ├── layout/              # 레이아웃 컴포넌트(제목 표시줄/사이드바/명령 팔레트)
│   ├── help/                # 도움말 패널
│   ├── onboarding/          # 온보딩 마법사
│   ├── notification/        # 알림 센터
│   ├── search/              # 세션 검색
│   ├── common/              # 공통 컴포넌트
│   └── shared/              # 공유 컴포넌트
│
├── pages/                    # 페이지 컴포넌트(22개 페이지)
│   ├── ChatPage.tsx
│   ├── KnowledgePage.tsx
│   ├── KnowledgeHubPage.tsx
│   ├── MemoryPage.tsx
│   ├── WorkflowPage.tsx
│   ├── WorkflowMarketplace.tsx
│   ├── GatewayPage.tsx
│   ├── GatewayLinkPage.tsx
│   ├── LinkPage.tsx
│   ├── FilesPage.tsx
│   ├── FineTunePage.tsx
│   ├── SkillsPage.tsx
│   ├── WikiPage.tsx
│   ├── WikiEditorPage.tsx
│   ├── WikiGraphPage.tsx
│   ├── LlmWikiPage.tsx
│   ├── LlmWikiEditorPage.tsx
│   ├── IngestPage.tsx
│   ├── QuickBarPage.tsx
│   ├── SettingsPage.tsx
│   └── DevTools/
│       ├── TraceExplorer.tsx
│       ├── BenchmarkRunner.tsx
│       └── ToolRecommender.tsx
│
├── hooks/                    # React hooks(10개)
├── lib/                      # 유틸리티 함수(Web Worker 포함)
├── types/                    # TypeScript 타입 정의(22개)
├── sdk/                      # SDK(Python SDK 포함)
└── i18n/                     # 11개 언어 번역
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

# 코드 포맷팅
npm run format

# CI 검사
npm run ci:check
```

---

## 프로젝트 구조

```
AxAgent/
├── src/                         # 프론트엔드 소스 (React + TypeScript)
│   ├── components/              # React 컴포넌트(24개 모듈)
│   │   ├── chat/               # 채팅 인터페이스(90+ 컴포넌트)
│   │   ├── workflow/           # 워크플로 편집기 컴포넌트
│   │   ├── gateway/            # API 게이트웨이 컴포넌트
│   │   ├── settings/           # 설정 패널(40+ 컴포넌트)
│   │   ├── terminal/           # 터미널 컴포넌트
│   │   ├── skill/              # 스킬 편집기 및 렌더러
│   │   ├── benchmark/          # 벤치마크
│   │   ├── decomposition/      # 스킬 분해
│   │   ├── files/              # 파일 관리
│   │   ├── fine-tune/          # LoRA 미세 조정
│   │   ├── link/               # 외부 링크
│   │   ├── llm-wiki/           # LLM Wiki
│   │   ├── proactive/          # 능동적 제안
│   │   ├── recommendation/     # 도구 추천
│   │   ├── wiki/               # Wiki 관리
│   │   ├── devtools/           # 개발자 도구
│   │   ├── style/              # 코드 스타일
│   │   ├── layout/             # 레이아웃 컴포넌트
│   │   ├── help/               # 도움말 패널
│   │   ├── onboarding/         # 온보딩 마법사
│   │   ├── notification/       # 알림 센터
│   │   ├── search/             # 세션 검색
│   │   ├── common/             # 공통 컴포넌트
│   │   └── shared/             # 공유 컴포넌트
│   ├── pages/                   # 페이지 컴포넌트(22개 페이지)
│   ├── stores/                  # Zustand 상태 관리
│   │   ├── domain/            # 코어 비즈니스 상태(6개 store)
│   │   ├── feature/           # 기능 모듈 상태(30+ store)
│   │   ├── devtools/          # 개발자 도구 상태(5개 store)
│   │   └── shared/            # 공유 상태(4개 store)
│   ├── hooks/                   # React hooks(10개)
│   ├── lib/                     # 유틸리티 함수(Web Worker 포함)
│   ├── types/                   # TypeScript 타입 정의(22개)
│   ├── sdk/                     # SDK(Python SDK 포함)
│   └── i18n/                    # 11개 언어 번역
│
├── src-tauri/                    # 백엔드 소스 (Rust)
│   ├── crates/                  # Rust workspace(10개 crates)
│   │   ├── agent/             # AI 에이전트 코어
│   │   ├── core/              # 데이터베이스, 암호화, RAG
│   │   ├── gateway/           # API 게이트웨이 서버
│   │   ├── plugins/           # 플러그인 시스템
│   │   ├── providers/         # 모델 프로바이더 어댑터
│   │   ├── runtime/           # 런타임 서비스
│   │   ├── tools/             # 도구 시스템
│   │   ├── trajectory/        # 메모리 및 학습
│   │   ├── telemetry/         # 추적 및 지표
│   │   └── migration/         # 데이터베이스 마이그레이션
│   └── src/                    # Tauri 진입점(70+ 명령 모듈)
│
├── extension/                  # 브라우저 확장(Wiki Clipper)
├── e2e/                        # Playwright E2E 테스트
├── scripts/                    # 빌드 및 도구 스크립트
└── website/                    # 프로젝트 웹사이트(VitePress)
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
