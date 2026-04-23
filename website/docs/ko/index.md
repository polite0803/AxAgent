---
layout: home
title: AxAgent — 오픈소스 AI 데스크톱 클라이언트 & 게이트웨이
titleTemplate: false

head:
  - - meta
    - name: description
      content: AxAgent은 AI 게이트웨이가 내장된 무료 오픈소스 AI 데스크톱 클라이언트입니다. OpenAI, Claude, Gemini, DeepSeek 등 멀티 모델 대화, MCP 서버, 지식 베이스, 개인정보 보호 우선.

hero:
  name: AxAgent
  text: 당신의 AI 데스크톱 어시스턴트
  tagline: 멀티 모델 채팅, MCP 도구, API 게이트웨이, 지식 베이스 — 모두 하나의 오픈소스 클라이언트에서
  image:
    src: /logo.png
    alt: AxAgent
  actions:
    - theme: brand
      text: 시작하기
      link: /ko/guide/getting-started
    - theme: alt
      text: 다운로드
      link: /ko/download
    - theme: alt
      text: GitHub
      link: https://github.com/polite0803/AxAgent

features:
  - icon: robot
    title: 멀티 모델 채팅
    details: OpenAI, Claude, Gemini, DeepSeek, Qwen 등 모든 호환 API에 연결. 멀티 키 로테이션, 스트리밍 출력, 사고 블록 지원.
  - icon: thunderbolt
    title: AI Agent (Beta)
    details: Agent 모드로 자율적인 작업 수행. 파일 읽기/쓰기, 명령어 실행, 코드 분석——3단계 권한 제어 + 작업 디렉토리 샌드박스로 안전하게 제어.
  - icon: api
    title: MCP 도구 호출
    details: Model Context Protocol 완전 구현. stdio, SSE, StreamableHTTP 지원. 원클릭으로 외부 도구 연결, @axagent/fetch 등 내장 MCP 도구 제공.
  - icon: cloud-server
    title: 내장 API 게이트웨이
    details: 로컬 OpenAI 호환 API 서버. Claude Code, Codex, Gemini CLI 등의 백엔드로 사용 가능. 키 관리, 속도 제한, SSL/TLS.
  - icon: book
    title: 지식 베이스 & RAG
    details: sqlite-vec를 이용한 로컬 벡터 임베딩. 프라이빗 문서 기반 AI 답변, 데이터는 로컬에서 벗어나지 않습니다.
  - icon: search
    title: 웹 검색
    details: Tavily, Zhipu WebSearch, Bocha 통합. 인용 출처가 포함된 검색 결과가 대화 컨텍스트에 자동 주입됩니다.
  - icon: edit
    title: 풍부한 콘텐츠 렌더링
    details: Markdown, LaTeX, Mermaid 다이어그램, D2 아키텍처 다이어그램, Monaco 코드 에디터(diff 미리보기), Artifact 패널.
  - icon: desktop
    title: 데스크톱 경험
    details: 전역 단축키, 시스템 트레이, 자동 시작, 항상 위에 표시, 다크/라이트 테마, 프록시 지원.
  - icon: lock
    title: 개인정보 & 보안
    details: 모든 데이터 로컬 저장. AES-256으로 API 키 암호화. 로컬 또는 WebDAV로 자동 백업. PNG/Markdown/JSON으로 내보내기.
---
