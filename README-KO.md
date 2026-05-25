[**English**](./README-EN.md) | [简体中文](./README.md) | [繁體中文](./README-ZH-TW.md) | [日本語](./README-JA.md) | **한국어** | [Français](./README-FR.md) | [Deutsch](./README-DE.md) | [Español](./README-ES.md) | [Русский](./README-RU.md) | [हिन्दी](./README-HI.md) | [العربية](./README-AR.md)

[![AxAgent](https://github.com/polite0803/AxAgent/blob/main/src/assets/image/logo.png?raw=true)](https://github.com/polite0803/AxAgent)

<p align="center">
  <a href="https://www.producthunt.com/products/axagent?embed=true&amp;amp&utm_source=badge-featured&amp;amp;&amp;#10;&amp;amp&amp;amp;;utm_medium=badge&amp;amp;#10&amp;amp;;utm_campaign=badge-axagent" target="_blank" rel="noopener noreferrer"><img alt="AxAgent - Lightweight, high-perf cross-platform AI desktop client | Product Hunt" width="250" height="54" src="https://api.producthunt.com/widgets/embed-image/v1/featured.svg?post_id=1118403&amp;theme=light&amp;t=1775627359538"></a>
</p>

<p align="center">
  <strong>크로스 플랫폼 AI 데스크톱/모바일 클라이언트 | 멀티 에이전트 협업 | 로컬 우선</strong>
</p>

<p align="center">
  <a href="https://github.com/polite0803/AxAgent/releases" target="_blank">
    <img src="https://img.shields.io/github/v/release/polite0803/AxAgent?style=flat-square" alt="Release">
  </a>
  <a href="https://github.com/polite0803/AxAgent/actions" target="_blank">
    <img src="https://img.shields.io/github/actions/workflow/status/polite0803/AxAgent/release.yml?style=flat-square" alt="Build">
  </a>
  <img src="https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux%20%7C%20Android%20%7C%20iOS-blue?style=flat-square" alt="Platform">
  <img src="https://img.shields.io/badge/license-AGPL--3.0-green?style=flat-square" alt="License">
</p>

---

## AxAgent란?

**AxAgent v2.0**는 고급 AI 에이전트 기능과 풍부한 개발자 도구를 결합한 종합적인 크로스 플랫폼 AI 데스크톱/모바일 애플리케이션입니다. 멀티 프로바이더 모델 지원, 자율 에이전트 실행, 시각적 워크플로 오케스트레이션, 로컬 지식 관리 및 내장 API 게이트웨이를 제공하며, **Windows / macOS / Linux / Android / iOS** 5개 플랫폼을 지원합니다.

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
- **Candle 추론 엔진** — 내장 Candle 로컬 추론, rerank/judge 인터페이스 지원, GGUF 주문형 다운로드
- **모델 관리** — 원격 모델 목록 가져오기, 사용자 지정 가능한 매개변수(temperature, max tokens, top-p 등)
- **스트리밍 출력** — 실시간 토큰 단위 렌더링, 접이식 사고 블록(Claude 확장 사고) 지원
- **멀티 모델 비교** — 여러 모델에 동시에 동일한 질문을 전송하고 나란히 비교
- **함수 호출** — 지원되는 모든 프로바이더에 걸친 구조화된 함수 호출
- **OpenAI Responses API** — OpenAI Responses 형식 전송 지원
- **실시간 API** — OpenAI 실시간 API 호환 WebSocket 이벤트 푸시
- **AI 이미지 생성** — DALL-E 3 및 Flux(Replicate), 다중 크기 프리셋(1:1/16:9/9:16/4:3), 네거티브 프롬프트
- **스마트 모델 라우팅** — 작업 유형별 자동 라우팅(코드 리뷰/요약/번역), 사용자 정의 라우팅 규칙
- **음성 통화** — OpenAI 실시간 API를 통한 실시간 음성 대화, 연결/말하기/듣기 상태 전환

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
- **에이전트 풀 패널** — 하위 에이전트/Worker/워크플로 단계 실시간 상태 시각화
- **에이전트 리플렉션 패널** — 작업 후 품질 평가, 효율성 분석, 오류 패턴, 개선 제안
- **전문가 선택기** — 전문가 역할 가져오기/내보내기/사용자 정의, 카테고리 필터링, 내장 프리셋
- **에이전트 계층 트리** — 에이전트 계층 및 협업 토폴로지 시각화
- **의도 분류기** — 사용자 입력 의도 유형 자동 식별
- **신념 상태 관리** — 에이전트의 컨텍스트 이해 상태 유지
- **목표 평가기** — 작업 목표 달성도 및 품질 평가
- **컨텍스트 윈도우 관리** — 컨텍스트 윈도우 지능형 관리, 토큰 사용량 최적화
- **프로젝트 메모리** — 세션 간 프로젝트 수준 지식 영속화
- **지식 베이스 관리** — 지식 베이스 CRUD 작업
- **노트 시스템** — 에이전트 내 구조화된 노트 저장 및 검색

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
- **협업 패널** — 실시간 협업 세션 관리, 초대 코드 공유, 참가자 역할(Owner/Editor/Viewer)
- **세션 공유** — 원클릭 공유 링크, 터미널/파일/모델 액세스 권한 구성

### ⭐ 스킬 시스템

- **스킬 마켓플레이스** — 커뮤니티 기여 스킬을 검색하고 설치할 수 있는 내장 마켓플레이스
- **스킬 생성** — 제안에서 자동으로 스킬 생성, Markdown 편집기 지원
- **스킬 진화** — 실행 피드백에 기반한 AI 구동 기존 스킬의 자동 분석 및 개선
- **스킬 진화 패널** — 진화 세대, 최적/평균 적합도, 수렴 상태 시각화
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
- **Self-RAG** — 자기 검색 증강 생성, 검색 필요성 및 결과 관련성 지능형 판단
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
- **플러그인 시스템** — OpenClaw 호환 3단계 플러그인 아키텍처(내장/번들/외부), npm 패키지 설치, 도구 등록, 훅 및 수명 주기 관리 지원
- **플러그인 마켓플레이스** — 내장 마켓플레이스 UI, npm 검색 설치 및 확인 대화상자 지원
- **내장 도구** — 종합적인 파일 작업(읽기/쓰기/편집), 코드 실행, 검색(Grep/Glob), Bash, 웹 검색, 웹 스크래핑, 계획 관리, Cron 스케줄링, REPL, LSP, 컨텍스트 관리, 컴퓨터 제어, 메시지 푸시, 할 일 등
- **도구 권한 시스템** — 도구 권한 분류, 규칙 관리 및 사용 추적
- **Bash 보안** — 명령 파싱, 경로 검증 및 샌드박스 보안 제어
- **LSP 클라이언트** — 내장 언어 서버 프로토콜, 코드 완성 및 진단 지원
- **AST 인덱스** — 코드 파일의 AST 파싱 및 인덱스 구축
- **터미널 백엔드** — 로컬, Docker 및 SSH 터미널 연결 지원
- **브라우저 자동화** — CDP를 통한 브라우저 제어 기능 통합(탐색, 스크린샷, 클릭, 폼 작성, 텍스트 추출 등)
- **UI 자동화** — 크로스 플랫폼 UI 요소 식별 및 제어
- **Git 도구** — 분기 감지 및 충돌 인식을 지원하는 Git 작업
- **Git 커밋 패널** — 시각적 Git diff 통계, AI 생성 커밋 메시지, 원클릭 스테이징 및 커밋
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
- **차트 인터프리터** — AI 차트 데이터 분석 및 시각화(막대/선/원/산점/영역), 자동 인사이트
- **Diff 뷰어** — 대화 버전 비교, 파일별 Accept/Reject, 자동 언어 감지
- **컨텍스트 분류 바** — 카테고리별 세그먼트 컨텍스트 토큰 사용량 표시
- **컨텍스트 그래프** — ReactFlow를 통한 컨텍스트 관계 시각화
- **명령어 제안** — 입력 중 명령어 자동 제안
- **인용 관리자** — 인용 출처 추적/분류 및 신뢰도 평가
- **신뢰도 배지** — 5점 신뢰도 시각화

### 🛡️ 데이터 및 보안

- **AES-256 암호화** — API 키 및 민감한 데이터는 AES-256-GCM으로 암호화
- **분리 저장소** — 애플리케이션 상태는 `~/.axagent/`에, 사용자 파일은 `~/Documents/axagent/`에 저장
- **자동 백업** — 로컬 디렉토리 또는 WebDAV 저장소로 예약된 백업
- **클라우드 워크스페이스** — S3 및 WebDAV 클라우드 스토리지 동기화, 충돌 감지/해결, 양방향 동기화
- **백업 복원** — 원클릭으로 이전 백업에서 복원
- **내보내기 옵션** — PNG 스크린샷, Markdown, 일반 텍스트, JSON 형식
- **저장소 관리** — 시각적 디스크 사용량 표시 및 정리 도구
- **파일 권한 부여** — 파일 액세스 권한 부여 및 취소 관리
- **작업 감사** — 주요 작업의 감사 로그 기록

### 🖥️ 데스크톱 환경

- **반응형 레이아웃** — 데스크톱/태블릿/모바일 3단계 자동 적응(600px/900px 브레이크포인트), 실시간 크기 조정 전환
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
- **드림 상태 표시기** — 드림 통합 상태 및 결과의 실시간 표시
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

### 🛡️ 프롬프트 인젝션 방어 (Prompt-Guard)

- **4단계 방어 체계** — L1 패턴 감지(고위험 차단 + 중위험 플래그) → L2 구분자 이스케이프 → L3 XML 래퍼 → L4 신뢰 태그
- **파이프라인 오케스트레이터** — 다단계 감지 파이프라인, 사용자 정의 위험 임계값 지원
- **토큰 스머글링 감지** — 인코딩 난독화 및 토큰 스머글링 공격 전용 감지
- **Strict 모드** — 엄격 모드 테스트 + 중위험 사유 명명 + 사용자 정의 모드 문서
- **전체 파이프라인 통합** — session / prompt / git / RAG 각 단계에 통합 완료

### 📱 모바일 지원

- **Android 네이티브** — APK/AAB 빌드, arm64-v8a / armeabi-v7a / x86_64 지원
- **iOS 네이티브** — IPA 빌드, arm64 지원
- **적응형 레이아웃** — 데스크톱/태블릿/스마트폰 3단계 자동 적응(600px/900px CSS 브레이크포인트, 실시간 창 크기 조정 전환)
- **모바일 내비게이션** — Drawer 슬라이드 내비게이션 + 하단 내비바 + 플래시 FAB
- **안전 영역 적응** — Android 시스템 상태바/내비바 CSS env() 적응
- **CSP 최적화** — Android WebView CSP 프로토콜 화이트리스트

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
| **인포그래픽** | @antv/infographic |
| **아이콘** | Iconify + Lucide |
| **드래그 앤 드롭** | @dnd-kit |
| **빌드** | Vite 8 + npm |
| **테스트** | Vitest + Playwright + cargo-nextest |
| **포맷팅** | dprint (TS/JSON) + rustfmt |
| **Lint** | TS: eslint + oxlint / Rust: clippy + cargo-deny |
| **모바일** | Tauri Android + iOS 네이티브 빌드 |
| **데스크톱** | Windows (MSI) · macOS (DMG) · Linux (AppImage/deb/rpm) |

### 플랫폼 지원

| 플랫폼 | 아키텍처 |
|--------|----------|
| Windows | x86_64, ARM64 |
| macOS | Apple Silicon (arm64), Intel (x86_64) |
| Linux | x86_64, ARM64 |
| Android | arm64-v8a, armeabi-v7a, x86_64 (에뮬레이터) |
| iOS | arm64 |

### Rust 백엔드 아키텍처

백엔드는 전문화된 **18개** crates로 구성된 Rust workspace로 구성됩니다:

```
src-tauri/crates/
├── agent/            # AI 에이전트 코어(ReAct 엔진, 조정, 계획, 심층 연구, 팩트체크 등)
├── core/             # 코어 유틸리티(데이터베이스, RAG, 암호화, MCP, 브라우저 자동화, AST 인덱스 등)
├── providers/        # 모델 프로바이더 어댑터(OpenAI, Anthropic, Gemini, Ollama, OpenClaw 등)
├── runtime-core/     # 런타임 추상화 레이어(공통 타입, trait 정의, 구성)
├── runtime/          # 런타임 서비스(세션 관리, MCP, 터미널, 속도 제한, Webhook, 권한 등)
├── rt-workflow/      # 워크플로 엔진(DAG 오케스트레이션, 노드 실행기, 스케줄러)
├── rt-messaging/     # 메시지 게이트웨이(DingTalk/Feishu/QQ/Slack/WeChat/WhatsApp/Telegram/Discord 통합)
├── rt-webhook/       # Webhook 서버 및 디스패치
├── rt-dashboard/     # 대시보드 플러그인 시스템
├── rt-theme/         # 테마 엔진
├── gateway/          # API 게이트웨이(HTTP 서버, 인증, 라우팅, OpenAI 호환 인터페이스)
├── tools/            # 도구 시스템(레지스트리, 오케스트레이션, 스트리밍 출력, 40+ 내장 도구)
├── trajectory/       # 학습 시스템(메모리, 스킬, RL, 사용자 프로파일링, 드림 통합)
├── telemetry/        # 원격 측정 및 분산 추적
├── plugins/          # 플러그인 시스템(OpenClaw 호환, npm 패키지 설치)
├── prompt-guard/     # 프롬프트 인젝션 방어(L1-L4 다단계 감지 및 방어)
├── migration/        # 데이터베이스 마이그레이션
├── npm/              # npm 패키지 파싱 및 레지스트리
└── code_engine/      # Candle 로컬 추론 엔진(사용 중단, 기능은 core에 통합됨)
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
npm run test          # Vitest watch
npm run test:run      # Vitest 단일 실행

# E2E 테스트
npm run test:e2e      # Playwright
npm run test:e2e:ui   # Playwright UI 모드

# Rust 백엔드 테스트
cd src-tauri && cargo nextest run   # cargo-nextest(2-3배 빠름)
cd src-tauri && cargo test          # 표준 테스트

# 타입 확인
npm run typecheck     # TypeScript
cd src-tauri && cargo clippy -- -D warnings  # Rust

# 코드 포맷팅
npm run format        # dprint
cd src-tauri && cargo fmt

# CI 전체 검사
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
│   ├── pages/                   # 페이지 컴포넌트(18개 페이지)
│   ├── stores/                  # Zustand 상태 관리(62개 store)
│   │   ├── domain/            # 코어 비즈니스 상태(9개)
│   │   ├── feature/           # 기능 모듈 상태(44개)
│   │   ├── devtools/          # 개발자 도구 상태(5개)
│   │   └── shared/            # 공유 상태(4개)
│   ├── hooks/                   # React hooks
│   ├── lib/                     # 유틸리티 함수(Web Worker 포함)
│   ├── types/                   # TypeScript 타입 정의
│   ├── sdk/                     # SDK(Python SDK 포함)
│   └── i18n/                    # 11개 언어 번역
│
├── src-tauri/                    # 백엔드 소스 (Rust)
│   ├── crates/                  # Rust workspace(18개 crates)
│   │   ├── agent/             # AI 에이전트 코어
│   │   ├── core/              # 데이터베이스, 암호화, RAG, MCP
│   │   ├── providers/         # 모델 프로바이더 어댑터
│   │   ├── runtime-core/      # 런타임 추상화 레이어
│   │   ├── runtime/           # 런타임 서비스
│   │   ├── rt-workflow/       # 워크플로 엔진
│   │   ├── rt-messaging/      # 메시지 게이트웨이
│   │   ├── rt-webhook/        # Webhook 서버
│   │   ├── rt-dashboard/      # 대시보드 플러그인
│   │   ├── rt-theme/          # 테마 엔진
│   │   ├── gateway/           # API 게이트웨이 서버
│   │   ├── tools/             # 도구 시스템
│   │   ├── trajectory/        # 메모리 및 학습
│   │   ├── telemetry/         # 추적 및 지표
│   │   ├── plugins/           # 플러그인 시스템
│   │   ├── prompt-guard/      # 프롬프트 인젝션 방어
│   │   ├── migration/         # 데이터베이스 마이그레이션
│   │   └── npm/               # npm 패키지 파싱
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
