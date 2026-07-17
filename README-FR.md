# AxAgent

[![Release](https://img.shields.io/github/v/release/polite0803/AxAgent?style=flat-square)](https://github.com/polite0803/AxAgent/releases)
[![Build](https://img.shields.io/github/actions/workflow/status/polite0803/AxAgent/release.yml?style=flat-square)](https://github.com/polite0803/AxAgent/actions)
![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux%20%7C%20Android%20%7C%20iOS-blue?style=flat-square)
![License](https://img.shields.io/badge/license-AGPL--3.0-green?style=flat-square)

<p align="center">
  <a href="./media/poster-axagent.svg">
    <img src="./media/poster-axagent.svg" alt="Affiche AxAgent" width="80%" />
  </a>
</p>

**AxAgent** est un client de bureau AI multiplateforme basé sur Tauri 2 (Windows / macOS / Linux / Android / iOS). Il intègre un moteur d'agent ReAct, un orchestrateur visuel de workflows, des bases de connaissances RAG locales, des extensions de protocole MCP, une passerelle multi-modèles unifiée, l'automatisation de navigateur et le contrôle d'ordinateur — servant de station de travail AI pour le développement quotidien, la recherche, la gestion des connaissances et l'automatisation.

> **Langues**: [简体中文](./README.md) | [English](./README-EN.md) | [繁體中文](./README-ZH-TW.md) | [日本語](./README-JA.md) | [한국어](./README-KO.md) | [Français](./README-FR.md) | [Deutsch](./README-DE.md) | [Español](./README-ES.md) | [Русский](./README-RU.md) | [हिन्दी](./README-HI.md) | [العربية](./README-AR.md)

---

## Positionnement du Projet

AxAgent résout trois problèmes fondamentaux :

1. **Accès Multi-Modèles Unifié et Routage Intelligent** — Utilisez OpenAI, Anthropic Claude, Google Gemini, les modèles locaux Ollama et toute API compatible OpenAI dans une seule interface, avec rotation automatique multi-clés par quota, routage intelligent par type de tâche et comparaison en streaming
2. **Boucle Fermée de la Conversation à l'Exécution** — 47+ outils intégrés + workflows visuels + extensions MCP + navigateur/contrôle d'ordinateur, l'IA peut manipuler des fichiers, exécuter du code, gérer Git et planifier des tâches
3. **Souveraineté des Données Locale-First** — Les conversations, bases de connaissances, mémoires et configurations sont stockées dans une base SQLite locale, les clés API sont chiffrées en AES-256-GCM. Les fonctionnalités principales fonctionnent sans services cloud tiers

---

## Capacités Principales

### Moteur Multi-Modèles

- **9 Adaptateurs de Fournisseurs**: OpenAI (Chat Completions + Responses + Realtime), Anthropic Claude, Google Gemini, Ollama (avec gestion de modèles locaux GGUF), OpenClaw, Hermes et toutes les API compatibles OpenAI
- **Rotation Multi-Clés**: Plusieurs clés API par fournisseur, rotation automatique par quota, basculement automatique en cas de limitation
- **Routage Intelligent**: Sélection automatique du modèle par type de tâche (revue de code / résumé / traduction / général), règles personnalisables
- **Surveillance de Santé des Fournisseurs**: Suivi en temps réel du taux de succès, latence et disponibilité, avec dégradation automatique par niveau
- **Génération d'Images IA**: DALL-E 3 et Flux (Replicate) avec préréglages multi-tailles
- **Voix en Temps Réel**: Conversation vocale WebSocket basée sur l'API Realtime d'OpenAI, avec interruption et transcription en streaming

### Système d'Agent (Moteur ReAct)

- **Planificateur Hiérarchique** (`hierarchical_planner`): Décomposition des tâches complexes en plans structurés Phase → Task, compilés en exécution topologique DAG
- **Recherche Approfondie** (`deep_research`): Orchestration de recherche multi-sources incluant planification, exécution, synthèse de contenu et suivi des citations
- **Vérificateur de Faits** (`fact_checker`): Vérification des faits pilotée par IA avec classificateur de sources et évaluation de crédibilité
- **Arbre de Pensées** (`tree_of_thoughts`): Exploration de raisonnement multi-chemins avec évaluation de branches et retour arrière
- **Réflecteur** (`reflector`): Auto-évaluation post-exécution et suggestions d'amélioration
- **Auto-Vérificateur** (`self_verifier`): Validation automatique des résultats de raisonnement avec détection de cycles
- **Récupération d'Erreurs** (`error_recovery_engine`): Classification du type d'erreur → sélection de stratégie → nouvelle tentative ou ajustement automatique, avec backoff exponentiel
- **Tests A/B** (`ab_testing`): Évaluation comparative de différentes stratégies de raisonnement
- **Système d'Évaluation** (`evaluator`): Framework de benchmarks intégré
- **Fine-Tuning LoRA** (`fine_tune`): Pipeline d'entraînement intégré avec gestion d'adaptateurs LoRA
- **Optimiseur RL** (`rl_optimizer`): Apprentissage par renforcement basé sur le feedback d'expérience

**Collaboration Multi-Agents**:

- Architecture de coordination maître-esclave avec exécution parallèle des sous-agents et ordonnancement sensible aux dépendances
- Tableau noir partagé pour l'échange d'informations entre agents
- Mode débat contradictoire (rounds Pour/Contre avec score de force des arguments)
- Mode Swarm pour clusters d'agents multi-processus
- Mode proactif : les agents peuvent initier des suggestions et opérations

**Contrôle d'Ordinateur**: Clics de souris, saisie clavier, défilement d'écran pilotés par IA, avec trois niveaux de permissions (défaut / accepter les modifications / accès complet) et isolation par sandbox

**Automatisation de Navigateur**: Contrôle du navigateur via le protocole CDP, navigation, captures d'écran, clics, remplissage de formulaires et extraction de texte

### Système de Compétences

- **Marketplace de Compétences**: Parcourir et installer des compétences communautaires
- **Création Assistée par IA**: Création automatique de structures de compétences à partir de propositions en langage naturel (`skill:create`)
- **Évolution des Compétences** (`evolution_engine`): Analyse et amélioration automatiques des compétences basées sur le feedback d'exécution
- **Correspondance Sémantique**: Recommandation contextuelle de compétences pertinentes
- **Décomposition de Compétences** (`skill_decomposition`): Décomposition automatique de tâches complexes en combinaisons de compétences atomiques
- **Outils Générés**: Nouveaux outils générés et enregistrés par l'IA
- **Exécution en Sandbox**: Les compétences s'exécutent dans des environnements sandbox isolés

### Workflow Visuel

Éditeur de workflow DAG par glisser-déposer basé sur ReactFlow 12 :

- **17 Types de Nœuds**: Déclencheur, Agent, Appel LLM, Branchement Conditionnel, Fork Parallèle, Boucle, Fusion, Délai, Appel d'Outil, Exécution de Code, Sous-Workflow, Recherche Vectorielle, Analyse de Document, Validation, Fin, Règle Métier, Rôle Agent
- **Exécution par Tri Topologique de Kahn**: Détection automatique des cycles, ordonnancement parallèle en pipeline
- **Templates Intégrés**: Revue de code, correction de bug, documentation, test, refactoring, exploration, analyse de performance, audit de sécurité, développement de fonctionnalité
- **Sérialisation YAML**: Import/export de définitions de workflow
- **Gestion de Versions**: Contrôle de version des templates
- **Conception Assistée par IA**: Conception de workflow et recommandation de nœuds assistées par IA

### Gestion des Connaissances

- **RAG Multi-Bases de Connaissances**: Upload de documents → analyse automatique (PDF/DOCX/XLSX/PPTX/TXT) → segmentation → indexation vectorielle
- **Recherche Hybride**: Similarité vectorielle (sqlite-vec + embeddings locaux candle) + recherche plein texte BM25 (FTS5), classement hybride
- **Self-RAG**: Réflexion et validation automatiques des résultats de recherche
- **Re-Ranking**: Re-classement des résultats par cross-encoder
- **Graphe de Connaissances**: Extraction d'entités → construction de relations → graphe visuel
- **Surveillance de Fichiers**: Surveillance en temps réel des modifications via `notify`, indexation incrémentale automatique
- **LLM Wiki**: Compilateur et validateur Wiki assisté par IA

### Système de Mémoire

- **Mémoire Multi-Espaces de Noms**: Isolation par projet/sujet, saisie manuelle et extraction automatique par IA
- **Intégration Persistante**: Mémoire en boucle fermée Honcho et Mem0
- **Profil Utilisateur**: Apprentissage automatique du style de code, préférences technologiques et style de communication
- **Transfert de Style**: Extraction des caractéristiques de style de code → application au code généré par IA
- **Intégration Dream**: Consolidation automatique en arrière-plan des fragments de mémoire et modèles comportementaux en connaissances structurées
- **Mémoire de Projet**: Persistance du contexte par projet

### Passerelle API

Passerelle HTTP + WebSocket intégrée basée sur `axum` :

- **Endpoints Compatibles**: OpenAI `/v1/chat/completions`, API Claude Messages, API Gemini, ainsi que OpenAI Responses et Realtime WebSocket
- **Gestion de Clés**: Génération, révocation, activation/désactivation de clés d'accès avec expiration
- **Suivi d'Utilisation**: Statistiques de requêtes et consommation de tokens par clé/fournisseur/date, export de métriques Prometheus
- **Limitation de Débit**: Algorithme de seau à jetons via `governor`
- **SSL/TLS**: Certificats auto-signés intégrés (`rcgen`), support de certificats personnalisés
- **Liaison Externe**: Intégration en un clic avec Claude CLI, OpenCode et autres outils externes, synchronisation automatique des clés API
- **Tickets Temps Réel**: Tickets d'authentification temporaires basés sur HMAC pour le transfert sécurisé de connexions WebSocket

### Intégration de Plateformes de Messagerie

Passerelle multi-plateforme via `rt-messaging`, supportant la réception de messages, l'analyse de commandes et la réponse automatique par IA pour **DingTalk, Feishu, QQ, Slack, WeChat, WhatsApp, Telegram et Discord**.

### Système d'Outils

47+ outils intégrés, enregistrés uniformément via le trait `Tool` :

| Catégorie             | Outils                                                                                                                                                                                                     |
| --------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Opérations Fichiers   | `file_read`, `file_write`, `file_edit`, `file_system`                                                                                                                                                      |
| Exécution de Code     | `bash`, `repl`                                                                                                                                                                                             |
| Recherche             | `grep`, `glob`                                                                                                                                                                                             |
| Navigateur            | `browser` (CDP)                                                                                                                                                                                            |
| Contrôle d'Ordinateur | `computer_use` (souris/clavier/capture d'écran)                                                                                                                                                            |
| Web                   | `web_search`, `web_fetch`                                                                                                                                                                                  |
| Base de Connaissances | `knowledge`, `document`                                                                                                                                                                                    |
| Git                   | `git` (commit/push/branch/diff)                                                                                                                                                                            |
| Outils Dev            | `lsp`, `workspace`                                                                                                                                                                                         |
| Gestion de Tâches     | `plan`, `task_system`, `todo_write`, `cron`                                                                                                                                                                |
| Messagerie            | `push_notification`, `messaging`                                                                                                                                                                           |
| Base de Données       | `database`                                                                                                                                                                                                 |
| Stockage              | `storage`                                                                                                                                                                                                  |
| Autres                | `agent`, `agent_memory`, `context`, `export`, `integration`, `media`, `media_delivery`, `migration_tool`, `monitor`, `obsidian`, `ocr`, `personality`, `shared_path`, `system_info`, `testing`, `worktree` |

### Protocole MCP

Implémentation complète du protocole MCP (Model Context Protocol) basée sur `rmcp` :

- **Transport**: sous-processus stdio + HTTP Streamable + WebSocket
- **Authentification OAuth**: Flux d'autorisation OAuth pour les serveurs MCP
- **Découverte d'Outils**: Découverte et enregistrement automatiques des outils exposés par les serveurs MCP
- **Gestionnaire MCP**: Gestion du cycle de vie, vérifications de santé, reconnexion automatique

### Système de Plugins

Architecture de plugins à trois niveaux compatible OpenClaw (intégrés / groupés / externes) :

- Installation de packages npm avec marketplace UI pour recherche et installation
- Définition de manifeste de plugin, déclaration de permissions, exécution en sandbox
- Enregistrement d'outils personnalisés, fournisseurs d'agents, interception de Hooks
- Installateur de compétences : installation de compétences depuis des packages de plugins

### Sécurité

- **Chiffrement AES-256-GCM**: Stockage local chiffré des clés API et configurations sensibles (crate `crypto`)
- **Protection contre l'Injection de Prompts**: Pipeline de défense à quatre niveaux (`prompt-guard`) — détection de motifs → échappement de délimiteurs → wrapper XML → labels de confiance, intégré sur toute la chaîne conversation/construction de prompts/Git/RAG
- **Protection SSRF**: Vérification de sécurité des URL pour bloquer les requêtes vers des adresses internes
- **Filtrage de Contenu**: Filtrage de sécurité multi-types
- **Limitation de Débit**: Limitation par seau à jetons pour les appels d'outils et requêtes API
- **Disjoncteur**: Coupure automatique en cas d'échecs consécutifs
- **Contrôle d'Accès**: Contrôle d'accès aux outils basé sur des politiques
- **Isolation Sandbox**: Isolation de l'environnement d'exécution pour les agents et compétences

### Outils Développeur

- **Traçage Distribué** (`telemetry`): Intégration OpenTelemetry avec visualisation Span/Trace
- **Logging Structuré**: tracing-subscriber + horodatage chrono
- **Débogage par Rejeu**: Enregistrement de trajectoire d'exécution d'agent (`trajectory_recorder`) et rejeu
- **Panneau DevTools**: Visionneuse de timeline Trace Explorer, Benchmark Runner, Tool Recommender
- **Benchmarks**: Benchmarks Criterion (tool_exec / llm_call / search)
- **Vérifications CI**: `npm run ci:check` intégrant vérification de types, linting et validation de format

### Expérience Desktop et Mobile

- **Layout Responsive**: Adaptation desktop/tablette/mobile par breakpoints CSS (3 niveaux : `desktop` / `tablet` / `mobile`)
- **11 Langues**: Chinois simplifié, Chinois traditionnel, Anglais, Japonais, Coréen, Français, Allemand, Espagnol, Russe, Hindi, Arabe
- **Moteur de Thèmes** (`rt-theme`): Thèmes sombre/clair + multiples préréglages (incluant le thème monospace 21th), personnalisation profonde Ant Design 6
- **Éditeur Monaco**: Coloration syntaxique, aperçu des différences, support multilingue
- **Terminal xterm.js**: WebLinks, Unicode 11, recherche
- **Défilement Virtuel**: @tanstack/react-virtual + react-virtuoso
- **Rendu Graphique**: D2 + Mermaid + Recharts
- **Menu de Copie Global**: Menu de copie personnalisé, suppression du menu contextuel natif
- **Palette de Commandes**: Palette de commandes globale Ctrl+K
- **Zone de Notification + Raccourcis Globaux + Démarrage Auto**: Fonctionnement en arrière-plan non intrusif
- **Mise à Jour Auto**: Vérification des versions GitHub Releases à intervalles configurables
- **Support Proxy**: Configuration proxy HTTP / SOCKS5
- **Espace de Travail Cloud**: Synchronisation de stockage S3 et WebDAV avec détection de conflits et synchronisation bidirectionnelle

### Mobile

- Android APK/AAB (arm64-v8a, armeabi-v7a, x86_64)
- iOS IPA (arm64)
- Adaptations spécifiques mobiles : insets de zone sécurisée, navigation inférieure, navigation par tiroir

---

## Architecture Technique

### Stack Technique

| Couche              | Technologie                              | Version |
| ------------------- | ---------------------------------------- | ------- |
| Framework Desktop   | Tauri                                    | 2.11    |
| Framework Frontend  | React                                    | 19      |
| Système de Types    | TypeScript                               | 7       |
| Bibliothèque UI     | Ant Design                               | 6       |
| Framework CSS       | TailwindCSS                              | 4       |
| Gestion d'État      | Zustand                                  | 5       |
| Routage             | React Router                             | 7       |
| Éditeur de Code     | Monaco Editor                            | 0.55    |
| Terminal            | xterm.js                                 | 6       |
| Éditeur de Workflow | ReactFlow                                | 12      |
| Graphiques          | D2 + Mermaid + Recharts                  |         |
| Animation           | Framer Motion                            | 12      |
| Défilement Virtuel  | @tanstack/react-virtual + react-virtuoso |         |
| Glisser-Déposer     | @dnd-kit                                 | 6       |
| Rendu Markdown      | markstream-react + stream-markdown       |         |
| i18n                | i18next + react-i18next                  |         |
| Outil de Build      | Vite                                     | 8       |
| Tests               | Vitest + Playwright                      |         |
| Formatage           | dprint (TS/JSON/Markdown/TOML) + rustfmt |         |
| Linting             | ESLint + Oxlint + Clippy                 |         |

### Architecture Backend: Injection de Dépendances Harness

Architecture Rust workspace avec **32 crates**, suivant le pattern **Harness DI** :

> Tous les crates sont découplés via les interfaces trait définies par axagent-harness, et axagent-runtime assemble et injecte les dépendances à l'exécution.
> Direction des dépendances : `implémentations concrètes → harness ← appelants`

**harness** est la pierre angulaire architecturale — zéro logique métier, zéro implémentation concrète, contenant uniquement des définitions de traits, des DTO de données pures, des constantes et des types d'erreur unifiés. Il est dépendu par tous les autres crates et ne dépend d'aucun crate axagent-* (200+ définitions de traits couvrant Agent/Provider/Tool/RAG/Stockage/MCP/Plugins/Sécurité/Observabilité/Mémoire/Apprentissage/Navigateur/Messagerie, etc.).

```
src-tauri/crates/
├── harness/          # Pierre angulaire architecturale — interfaces trait, DTO, types d'erreur, contrats DI
├── entities/         # Modèles d'entités SeaORM
├── dao/              # Couche d'accès aux données (CRUD)
├── migration/        # Migrations de base de données
├── crypto/           # Chiffrement/déchiffrement AES-256-GCM et gestion de clés
├── credential/       # Stockage sécurisé des credentials
├── storage/          # Abstraction de stockage de fichiers (local/S3/WebDAV), lecture/écriture ZIP
├── cache/            # Couche de cache en mémoire
├── disk-cache/       # Cache de fichiers sur disque
├── search/           # Moteur de recherche (FTS5 + sqlite-vec + embeddings locaux candle)
├── document-parser/  # Extraction de texte de documents (PDF/DOCX/XLSX/PPTX)
├── kit/              # Utilitaires généraux (chemins/encodage/hachage/dates)
├── runtime-core/     # Types communs d'exécution, constantes de configuration
├── runtime/          # Orchestration des services d'exécution — conteneur DI assemblant tous les 30+ crates
├── rt-workflow/      # Moteur de workflow — orchestration DAG, exécuteurs de nœuds, sérialisation YAML
├── rt-messaging/     # Passerelle de plateformes de messagerie — DingTalk/Feishu/QQ/Slack/WeChat/WhatsApp/Telegram/Discord
├── rt-webhook/       # Serveur webhook générique
├── rt-dashboard/     # Framework de plugins de tableau de bord
├── rt-theme/         # Moteur de thèmes
├── agent/            # Cœur d'agent IA — 80+ modules
│                     #   MoteurReAct/PlanificationHiérarchique/RechercheApprofondie/VérificationFaits/ArbreDePensées/
│                     #   Réflexion/AutoVérification/RécupérationErreurs/OptimisationRL/FineTuningLoRA/
│                     #   Évaluation/RecommandationOutils/TestsAB/Coordinateur/TableauNoir/PipelineVision/
│                     #   RechercheWeb/RechercheAcadémique/CompilationWiki, etc.
├── orchestrator/     # Orchestration d'agents — ordonnancement multi-agents, décomposition DAG, exécution de sous-graphes dynamiques
├── providers/        # Adaptateurs de fournisseurs de modèles
├── tools/            # Système d'outils — trait Tool/registre/orchestration/streaming/sandbox/47+ outils intégrés
├── gateway/          # Passerelle API — serveur HTTP/WS axum, OAuth, limitation de débit, Prometheus
├── mcp/              # Protocole MCP — stdio + HTTP Streamable, basé sur rmcp
├── trajectory/       # Système d'apprentissage — mémoire/évolution des compétences/profils utilisateur/intégration dream
├── plugins/          # Système de plugins — compatible OpenClaw, installation de packages npm, marketplace
├── telemetry/        # Observabilité — OpenTelemetry, logging structuré, métriques d'exécution
├── prompt-guard/     # Protection contre l'injection de prompts — pipeline de détection multi-niveaux L1-L4
├── npm/              # Client de registre npm
└── schema-gen/       # Outil de génération de schéma de base de données
```

### Architecture Frontend

```
src/
├── pages/            # Pages (23+ incluant sous-pages)
│   ├── ChatPage           # Interface de chat — barre latérale/flux de messages/panneau Agent/multi-onglets
│   ├── DashboardPage      # Tableau de bord — statistiques d'utilisation/distribution des modèles/graphiques de tendance
│   ├── WorkflowPage       # Éditeur de workflow — visualisation DAG ReactFlow
│   ├── KnowledgeHubPage   # Gestion des bases de connaissances — upload/indexation/recherche
│   ├── MemoryPage         # Gestion de la mémoire
│   ├── SkillsPage         # Marketplace de compétences
│   ├── SettingsPage       # Panneau de paramètres — 40+ éléments de configuration
│   ├── TerminalPage       # Terminal intégré — xterm.js
│   ├── FilesPage          # Gestion de fichiers
│   ├── GatewayLinkPage    # Passerelle API et gestion des liaisons externes
│   ├── QuickBarPage       # Barre rapide (fenêtre indépendante)
│   ├── DynamicUIManagerPage / DynamicPageViewer  # Moteur UI dynamique
│   ├── WikiGraphPage / WikiEditPage / IngestPage # LLM Wiki
│   ├── LearningGraphPage  # Graphe d'apprentissage
│   ├── FineTunePage       # Fine-tuning LoRA
│   ├── PersonaPage        # Gestion des personas
│   ├── WorkflowMarketplace # Marketplace de workflows
│   ├── DevTools/          # TraceExplorer / BenchmarkRunner / ToolRecommender
│   └── Workflow/          # WorkflowListPage
│
├── components/       # 28 modules, 450+ composants
│   ├── chat/         # Chat (flux de messages/saisie/ChatView/TabBar/RightPanel/pièces jointes/rendu d'appels d'outils)
│   ├── layout/       # Layout — 17 composants
│   │                 #   AppInitializer / Sidebar / ContentArea / TitleBar /
│   │                 #   CommandPalette / GlobalCopyMenu / GlobalErrorBoundary /
│   │                 #   GlobalStatusBar / ErrorNotificationToast / AppHeader /
│   │                 #   BackendStatusIndicator / IpcReconnectBanner /
│   │                 #   ModuleErrorBoundary / NotificationBell / UserProfileModal etc.
│   ├── agent/        # Panneau Agent/entrée/mini-panneau
│   ├── workflow/     # Éditeur de workflow (nœuds/arêtes/panneaux/templates/assistance IA)
│   ├── settings/     # Panneau de paramètres (40+ sous-composants)
│   ├── skill/        # Éditeur de compétences/rendu/panneaux flottants
│   ├── dynamicUI/    # Registre de composants UI dynamiques (26 composants intégrés)
│   ├── gateway/      # Gestion de passerelle API
│   ├── files/        # Gestion de fichiers
│   ├── terminal/     # Composants de terminal
│   ├── search/       # Interface de recherche
│   ├── benchmark/    # Panneau de benchmarks
│   ├── decomposition/# Décomposition de compétences et génération d'outils
│   ├── devtools/     # Timeline Trace/Span + panneau RL Training
│   ├── approval/     # UI de workflow d'approbation
│   ├── recommendation/ # Recommandation d'outils/modèles
│   ├── onboarding/   # WelcomeWizard / InteractiveTutorial
│   ├── help/         # Panneau d'aide
│   ├── notification/ # Composants de notification
│   ├── proactive/    # Suggestions proactives
│   ├── llm-wiki/     # Composants LLM Wiki
│   ├── wiki/         # Composants Wiki
│   ├── fine-tune/    # UI de fine-tuning
│   ├── trace/        # Composants Trace
│   ├── style/        # Style/thème
│   ├── shared/       # Composants partagés (ErrorBoundary / PageContextProvider)
│   └── common/       # Composants communs (Icon, etc.)
│
├── stores/           # Gestion d'état Zustand
│   ├── domain/       # 10 stores métier principaux (conversation/flux/compression/préférences/multi-modèles, etc.)
│   ├── feature/      # 48 stores de modules fonctionnels (agent/workflow/connaissances/compétences/passerelle/mémoire/terminal, etc.)
│   └── devtools/     # 4 stores d'outils développeur
│
├── hooks/            # React Hooks (raccourcis/palette de commandes/responsive/barre de défilement/thème/avatar, etc.)
├── lib/              # Bibliothèque d'utilitaires (invoke/pageRegistry/shortcuts/skillLifecycle/
│                     #   chartGenerator/codeExecutor/tokenEstimator/workflowLayout etc. — 45+ modules)
├── types/            # Définitions de types TypeScript
├── theme/            # Moteur de thème Shadcn
├── i18n/             # Fichiers de traduction en 11 langues (zh-CN/zh-TW/en-US/ja/ko/fr/de/es/ru/hi/ar)
├── constants/        # Constantes et flags de fonctionnalités
└── sdk/              # SDK d'intégration externe
```

### Flags de Fonctionnalités

Le projet gère le déploiement progressif de fonctionnalités via `featureFlags.ts` :

| Flag                | Statut | Description                                               |
| ------------------- | ------ | --------------------------------------------------------- |
| `AGENT_IN_THE_LOOP` | ✅     | Panneau Agent global + injection de contexte de page      |
| `DYNAMIC_UI`        | ✅     | Moteur de construction UI dynamique                       |
| `SELF_EVOLUTION_UI` | ❌     | Panneau de contrôle d'auto-évolution frontend             |
| `NL_EXTENSION`      | ❌     | Extensions métier dynamiques pilotées par langage naturel |

### Plugins Tauri

| Plugin              | Utilité                                     |
| ------------------- | ------------------------------------------- |
| `autostart`         | Démarrage automatique                       |
| `clipboard-manager` | Lecture/écriture du presse-papier           |
| `dialog`            | Boîtes de dialogue de sélection de fichiers |
| `fs`                | Accès au système de fichiers                |
| `global-shortcut`   | Enregistrement de raccourcis globaux        |
| `notification`      | Notifications système                       |
| `opener`            | Ouverture de liens/fichiers externes        |
| `process`           | Gestion de processus                        |
| `updater`           | Mise à jour automatique                     |
| `mcp-bridge`        | Pont de protocole MCP (non-Android)         |

---

## Répertoire de Données

```
~/.axagent/                    # Configuration de l'application
├── axagent.db                 # Base de données principale SQLite (SeaORM)
├── master.key                 # Clé maîtresse AES-256
├── vector_db/                 # Index vectoriel sqlite-vec
└── ssl/                       # Certificats SSL auto-signés

~/Documents/axagent/          # Fichiers utilisateur
├── images/                   # Pièces jointes images
├── files/                    # Pièces jointes fichiers
└── backups/                  # Sauvegardes automatiques
```

---

## Démarrage Rapide

### Prérequis

- [Node.js](https://nodejs.org/) 20+
- [Rust](https://www.rust-lang.org/) 1.75+ (edition 2024)
- [npm](https://www.npmjs.com/) 10+
- Windows: [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) (MSVC + Windows SDK)
- macOS: Xcode Command Line Tools
- Linux: `build-essential` + `libwebkit2gtk-4.1-dev` + `libssl-dev`

### Développement

```bash
git clone https://github.com/polite0803/AxAgent.git
cd AxAgent
npm install
npm run tauri dev      # Mode développement (Vite HMR + fenêtre Tauri)
```

### Build

```bash
npm run tauri build    # Build production desktop

npm run tauri:android:build   # Build Android
npm run tauri:ios:build       # Build iOS
```

Les artefacts de build desktop se trouvent dans `src-tauri/target/release/`.

### Tests

```bash
npm run test           # Tests unitaires frontend (Vitest watch)
npm run test:run       # Tests unitaires frontend (exécution unique)
npm run test:e2e       # Tests E2E (Playwright)

# Tests backend Rust
cd src-tauri && cargo nextest run
cd src-tauri && cargo test

# Vérification de types & Lint
npm run typecheck
cd src-tauri && cargo clippy -- -D warnings
npm run format         # Formatage dprint
npm run lint:eslint    # Vérification ESLint
npm run contracts      # Vérification de contrat API

# Vérification CI complète
npm run ci:check
```

### Scripts

| Commande                 | Utilité                                      |
| ------------------------ | -------------------------------------------- |
| `npm run bump`           | Mise à niveau interactive de version         |
| `npm run docs`           | Génération de documentation TypeDoc          |
| `npm run skill:create`   | Création d'un nouveau scaffold de compétence |
| `npm run skill:validate` | Validation de définition de compétence       |
| `npm run check:types`    | Vérification de cohérence des types          |

---

## Support Plateformes

| Plateforme | Architecture                          |
| ---------- | ------------------------------------- |
| Windows    | x86_64, ARM64                         |
| macOS      | Apple Silicon (arm64), Intel (x86_64) |
| Linux      | x86_64, ARM64                         |
| Android    | arm64-v8a, armeabi-v7a, x86_64        |
| iOS        | arm64                                 |

---

## Licence

Ce projet est open-source sous la licence [AGPL-3.0-only](LICENSE).

---

## Remerciements

AxAgent est construit sur de nombreux projets open-source exceptionnels :

- [Tauri](https://tauri.app/) — Framework desktop multiplateforme
- [React](https://react.dev/) + [Ant Design](https://ant.design/) — UI frontend
- [SeaORM](https://www.sea-ql.org/SeaORM/) — ORM Rust
- [sqlite-vec](https://github.com/asg017/sqlite-vec) — Recherche vectorielle
- [candle](https://github.com/huggingface/candle) — Inférence d'embeddings locale
- [rmcp](https://github.com/nicholasxjy/rmcp) — SDK MCP Rust
- [ReactFlow](https://reactflow.dev/) — Éditeur de workflow visuel
- [axum](https://github.com/tokio-rs/axum) — Framework HTTP
- [Monaco Editor](https://microsoft.github.io/monaco-editor/) — Éditeur de code
- [xterm.js](https://xtermjs.org/) — Émulateur de terminal
- [Zustand](https://zustand.docs.pmnd.rs/) — Gestion d'état
- [Framer Motion](https://www.framer.com/motion/) — Bibliothèque d'animation
- [Recharts](https://recharts.org/) — Bibliothèque de graphiques
