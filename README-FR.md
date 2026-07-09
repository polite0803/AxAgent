# AxAgent

[![Release](https://img.shields.io/github/v/release/polite0803/AxAgent?style=flat-square)](https://github.com/polite0803/AxAgent/releases)
[![Build](https://img.shields.io/github/actions/workflow/status/polite0803/AxAgent/release.yml?style=flat-square)](https://github.com/polite0803/AxAgent/actions)
![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux%20%7C%20Android%20%7C%20iOS-blue?style=flat-square)
![License](https://img.shields.io/badge/license-AGPL--3.0-green?style=flat-square)

**AxAgent** est un client de bureau AI assistant open source multiplateforme, prenant en charge **Windows / macOS / Linux / Android / iOS**. Bien plus qu'une simple interface de chat, il intègre un moteur d'agent ReAct, une orchestration visuelle de workflows, une base de connaissances RAG locale, l'extension du protocole MCP, une passerelle multi-modèles unifiée, l'automatisation du navigateur, le contrôle de l'ordinateur, et bien plus encore, servant de poste de travail AI pour le développement quotidien, la recherche, la gestion des connaissances et l'automatisation.

> **Langues** : [简体中文](./README.md) | [English](./README-EN.md) | [繁體中文](./README-ZH-TW.md) | [日本語](./README-JA.md) | [한국어](./README-KO.md) | [Français](./README-FR.md) | [Deutsch](./README-DE.md) | [Español](./README-ES.md) | [Русский](./README-RU.md) | [हिन्दी](./README-HI.md) | [العربية](./README-AR.md)

---

## Positionnement du projet

AxAgent résout trois problèmes fondamentaux :

1. **Orchestration multi-modèles unifiée** : Utilisez simultanément OpenAI, Anthropic Claude, Google Gemini, les modèles locaux Ollama et toute API compatible OpenAI dans une interface unique, avec rotation multi-clés, routage intelligent des modèles et comparaison en streaming
2. **L'IA passe de la conversation à l'exécution** : Grâce à plus de 47 outils intégrés, des workflows visuels, l'extension MCP, l'automatisation du navigateur et le contrôle de l'ordinateur, l'IA peut manipuler des fichiers, exécuter du code, gérer Git et orchestrer des tâches
3. **Souveraineté des données locale d'abord** : Les conversations IA, la base de connaissances, la mémoire et les fichiers de configuration sont stockés dans une base de données SQLite locale, les clés API sont chiffrées avec AES-256-GCM, les fonctions principales fonctionnent sans services cloud tiers

---

## Fonctionnalités principales

### Moteur multi-modèles

- **9 adaptateurs de fournisseurs** : OpenAI (Chat Completions + Responses + Realtime), Anthropic Claude, Google Gemini, Ollama (avec gestion GGUF), OpenClaw, Hermes, et toute API compatible OpenAI
- **Rotation multi-clés** : Configurez plusieurs clés API pour le même fournisseur, rotation automatique selon les quotas pour éviter les interruptions de limite de débit
- **Routage intelligent** : Sélection automatique du modèle le plus adapté selon le type de tâche (revue de code / résumé / traduction / général), règles de routage personnalisables
- **Surveillance de santé des fournisseurs** : Suivi en temps réel du taux de succès, de la latence et de la disponibilité de chaque fournisseur, dégradation automatique hiérarchisée (ProviderTier)
- **Génération d'images IA** : DALL-E 3 et Flux (Replicate) avec préréglages multi-tailles
- **Voix en temps réel** : Conversation vocale WebSocket basée sur l'API OpenAI Realtime, avec interruption et transcription en streaming

### Système d'agents

L'ensemble du système d'agents est construit sur le **moteur ReAct (Reasoning + Acting)** et comprend les sous-systèmes implémentés suivants :

- **Planificateur hiérarchique** (`hierarchical_planner`) : Décomposition des tâches complexes en plans structurés Phase → Tâche avec dépendances, compilés en exécution topologique DAG
- **Recherche approfondie** (`deep_research`) : Orchestration de recherche multi-sources (plan de recherche (`search_planner`), exécution de recherche (`search_orchestrator`), synthèse de contenu (`content_synthesizer`), suivi de citations (`citation_tracker`))
- **Vérificateur de faits** (`fact_checker`) : Vérification factuelle pilotée par l'IA (classificateur de sources (`source_classifier`), validateur de sources (`source_validator`), évaluateur de crédibilité (`credibility_evaluator`))
- **Arbre de pensées** (`tree_of_thoughts`) : Exploration de raisonnement multi-chemins, évaluation de branches et retour arrière
- **Réflecteur** (`reflector`) : Auto-évaluation post-exécution et génération de suggestions d'amélioration
- **Auto-vérificateur** (`self_verifier`) : Validation automatique des résultats de raisonnement, détection de cycles (`cycle_detector`) pour éviter le raisonnement infini
- **Récupération d'erreurs** (`error_recovery_engine`) : Classification du type d'erreur → sélection de la stratégie de récupération → nouvelle tentative automatique ou ajustement du plan, avec backoff exponentiel
- **Test A/B** (`ab_testing`) : Évaluation comparative de différentes stratégies de raisonnement
- **Système d'évaluation** (`evaluator`) : Cadre de benchmarks intégré (jeux de données, métriques, génération de rapports)
- **Fine-tuning LoRA** (`fine_tune`) : Pipeline d'entraînement intégré, gestion des adaptateurs LoRA
- **Optimiseur RL** (`rl_optimizer`) : Apprentissage par renforcement de politique basé sur le retour d'expérience (relecture d'expérience, gradient de politique)
- **Recommandeur d'outils** (`tool_recommender`) : Analyse et recommandation de patterns d'utilisation d'outils basées sur le contexte

**Collaboration multi-agents** :

- Architecture de coordination maître-esclave (`coordinator`), exécution parallèle des agents enfants, ordonnancement sensible aux dépendances
- Tableau noir partagé (`shared_blackboard`) pour l'échange d'informations entre agents
- Mode de débat contradictoire, tours Pro/Contre et score de force des arguments
- Mode cluster Swarm, cluster d'agents multi-processus avec synchronisation des autorisations et reconnexion automatique
- Mode proactif (`proactive_mode`) : Les agents peuvent lancer proactivement des suggestions et des actions

**Contrôle de l'ordinateur** : Clics de souris, saisie clavier, défilement d'écran pilotés par l'IA, trois niveaux de permissions (par défaut / accepter les modifications / accès complet), isolation par sandbox

**Automatisation du navigateur** : Contrôle du navigateur via le protocole CDP, avec navigation, captures d'écran, clics, remplissage de formulaires, extraction de texte, surveillance de l'état des pages

### Système de compétences

- **Marketplace de compétences** : Parcourir et installer des compétences communautaires
- **Création assistée par IA** : Création automatique de la structure de compétence à partir de propositions en langage naturel
- **Évolution des compétences** (`evolution_engine`) : Analyse et amélioration automatiques des compétences basées sur le retour d'exécution
- **Correspondance sémantique** (`skill`) : Correspondance sémantique des compétences pertinentes selon le contexte de conversation, recommandation automatique
- **Décomposition de compétences** (`skill_decomposition`) : Décomposition automatique des tâches complexes en combinaisons de compétences atomiques
- **Outil généré** (`generated_tool`) : Génération et enregistrement de nouveaux outils par l'IA
- **Exécution en sandbox** (`sandbox`) : Exécution sécurisée des compétences dans un environnement sandbox isolé

### Workflows visuels

Éditeur de workflow DAG par glisser-déposer basé sur ReactFlow 12 :

- **17 types de nœuds** : Déclencheur, Agent, Appel LLM, Branche conditionnelle, Fork parallèle, Boucle, Fusion, Délai, Appel d'outil, Exécution de code, Sous-workflow, Recherche vectorielle, Analyse de document, Validation, Fin, Règle métier, Rôle d'agent
- **Exécution par tri topologique de Kahn** : Détection automatique des dépendances circulaires, ordonnancement parallèle en pipeline
- **Modèles intégrés** : Revue de code, Correction de bug, Génération de documentation, Tests, Refactoring, Exploration, Analyse de performance, Audit de sécurité, Développement de fonctionnalité
- **Sérialisation YAML** : Import/export des définitions de workflow au format YAML
- **Gestion de versions** : Contrôle de version des modèles de workflow
- **Assistance IA** : Conception de workflow assistée par IA et recommandation de nœuds

### Gestion des connaissances

- **RAG multi-bases de connaissances** : Téléchargement de documents → analyse automatique (PDF/DOCX/XLSX/PPTX/TXT) → découpage en chunks → indexation vectorielle
- **Recherche hybride** : Similarité vectorielle (sqlite-vec + embeddings locaux candle) + recherche plein texte BM25 (FTS5), classement hybride
- **Self-RAG** : Génération augmentée par récupération auto-réflexive, réflexion et validation automatiques des résultats de recherche
- **Reclassement** : Reclassement des résultats par cross-encoder pour améliorer la précision
- **Graphe de connaissances** : Extraction d'entités (`EntityExtractor`) → construction de relations → graphe visuel
- **Surveillance de fichiers** : Surveillance en temps réel des modifications de fichiers basée sur `notify`, indexation incrémentale automatique
- **LLM Wiki** : Compilateur et validateur Wiki assistés par IA, extension de navigateur Wiki Cropping

### Système de mémoire

- **Mémoire multi-espaces de noms** : Isolation par projet/sujet, saisie manuelle et extraction automatique par IA
- **Intégration de persistance** : Mémoire en boucle fermée Honcho et Mem0
- **Profil utilisateur** (`user_profile` / `profile`) : Apprentissage automatique du style de code (indentation/nommage/commentaires), des préférences de stack technique et du style de communication
- **Transfert de style** (`style`) : Extraction des caractéristiques de style de code → application au code généré par l'IA
- **Intégration onirique** (`dream`) : Intégration automatique en arrière-plan des fragments de mémoire et des modèles comportementaux, génération de connaissances structurées
- **Mémoire de projet** (`project_memory`) : Persistance du contexte au niveau du projet

### Passerelle API

Serveur de passerelle HTTP + WebSocket intégré basé sur `axum` :

- **Points de terminaison compatibles** : OpenAI `/v1/chat/completions`, API Claude Messages, API Gemini, ainsi qu'OpenAI Responses et Realtime WebSocket
- **Gestion des clés** : Génération, révocation, activation/désactivation des clés d'accès, avec expiration configurable
- **Suivi d'utilisation** : Statistiques de volume de requêtes et de consommation de tokens par clé, fournisseur et date, export de métriques Prometheus
- **Limitation de débit** : Algorithme de seau à jetons basé sur `governor`, politiques de limitation de débit configurables
- **SSL/TLS** : Certificats auto-signés intégrés (`rcgen`), prise en charge de certificats personnalisés
- **Liens externes** : Intégration en un clic avec des outils externes tels que Claude CLI, OpenCode, synchronisation automatique des clés API
- **Tickets en temps réel** : Tickets d'authentification temporaires basés sur HMAC pour la transmission sécurisée des connexions WebSocket en temps réel

### Intégration de plateformes de messagerie

Passerelle de plateforme de messagerie implémentée via le crate `rt-messaging`, prenant en charge :

DingTalk, Feishu, QQ, Slack, WeChat, WhatsApp, Telegram, Discord

Réception de messages Webhook, analyse de commandes, relais automatique des réponses IA.

### Système d'outils

47 outils intégrés, tous enregistrés via le trait `Tool` :

| Catégorie                | Outils                                                                                                                                                                                                     |
| ------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Opérations sur fichiers  | `file_read`, `file_write`, `file_edit`, `file_system` (liste/recherche/métadonnées)                                                                                                                        |
| Exécution de code        | `bash`, `repl`                                                                                                                                                                                             |
| Recherche                | `grep`, `glob`                                                                                                                                                                                             |
| Navigateur               | `browser` (contrôle CDP)                                                                                                                                                                                   |
| Contrôle de l'ordinateur | `computer_use` (souris/clavier/capture d'écran)                                                                                                                                                            |
| Web                      | `web_search`, `web_fetch`                                                                                                                                                                                  |
| Base de connaissances    | `knowledge`, `document` (analyse de documents)                                                                                                                                                             |
| Git                      | `git` (commit/push/branch/diff)                                                                                                                                                                            |
| Outils de développement  | `lsp` (Language Server Protocol), `workspace`                                                                                                                                                              |
| Gestion de tâches        | `plan`, `task_system`, `todo_write`, `cron`                                                                                                                                                                |
| Notifications push       | `push_notification`, `messaging`                                                                                                                                                                           |
| Base de données          | `database`                                                                                                                                                                                                 |
| Stockage                 | `storage`                                                                                                                                                                                                  |
| Autres                   | `agent`, `agent_memory`, `context`, `export`, `integration`, `media`, `media_delivery`, `migration_tool`, `monitor`, `obsidian`, `ocr`, `personality`, `shared_path`, `system_info`, `testing`, `worktree` |

### Protocole MCP

Implémentation complète du protocole MCP (Model Context Protocol) basée sur le crate `rmcp` :

- **Couche de transport** : Sous-processus stdio + HTTP Streamable + WebSocket
- **Authentification OAuth** : Prise en charge du flux d'autorisation OAuth pour les serveurs MCP
- **Découverte d'outils** : Découverte et enregistrement automatiques des outils exposés par les serveurs MCP
- **Gestionnaire MCP** : Gestion du cycle de vie des serveurs, vérifications de santé, reconnexion automatique

### Système de plugins

Architecture de plugins à trois niveaux compatible OpenClaw (intégrés / groupés / externes), prenant en charge :

- Installation de packages npm, UI de marketplace intégrée pour la recherche et l'installation
- Définition de manifeste de plugin, déclaration de permissions, exécution isolée en sandbox
- Enregistrement d'outils personnalisés, fournisseurs d'agents, interception par hooks
- Installateur de compétences : Installation de compétences depuis les packages de plugins dans le système de compétences

### Sécurité

- **Chiffrement AES-256-GCM** : Stockage local chiffré des clés API et des paramètres sensibles (crate `crypto`)
- **Protection contre l'injection de prompts** : Pipeline de défense à quatre niveaux (`prompt-guard`) — détection de motifs → échappement des délimiteurs → encapsulation XML → balises de confiance, intégré à toute la chaîne (sessions, construction de prompts, Git, RAG)
- **Protection SSRF** (`ssrf_guard`) : Vérification de sécurité des URL, blocage des requêtes vers les adresses réseau internes
- **Filtrage de contenu** (`content_filter`) : Filtrage de sécurité de contenu multi-types
- **Limiteur de débit** (`rate_limiter`) : Limitation par seau à jetons des appels d'outils et des requêtes API
- **Disjoncteur** (`circuit_breaker`) : Coupure automatique en cas d'échecs consécutifs, protégeant la stabilité du système
- **Contrôle d'accès** (`tool_access`) : Contrôle des permissions d'accès aux outils basé sur des politiques
- **Isolation par sandbox** : Isolation de l'environnement d'exécution des agents et des compétences

### Expérience développeur

- **Traçage distribué** (`telemetry`) : Intégration OpenTelemetry, visualisation Span/Trace
- **Télémétrie** (`telemetry`) : Journalisation structurée, métriques d'exécution, collecte d'événements de performance
- **Débogage par relecture** : Enregistrement des trajectoires d'exécution des agents (`trajectory_recorder`) et relecture
- **Panneau DevTools** : Visualiseur de chronologie Trace/Span intégré dans le frontend
- **Cadre de benchmarks** : Benchmarks Criterion (tool_exec / llm_call / search), évaluation SWE-bench et Terminal-bench

### Expérience bureau et mobile

- **Mise en page responsive** : Adaptation aux points de rupture CSS pour desktop / tablette / mobile (600px / 900px)
- **11 langues** : Chinois simplifié, Chinois traditionnel, Anglais, Japonais, Coréen, Français, Allemand, Espagnol, Russe, Hindi, Arabe
- **Moteur de thèmes** (`rt-theme`) : Thème sombre/clair, suivant le système ou commutation manuelle, personnalisation approfondie Ant Design 6
- **Éditeur Monaco** : Éditeur de code intégré avec coloration syntaxique, aperçu des différences, multilangue
- **Terminal xterm.js** : Émulateur de terminal intégré avec WebLinks, Unicode 11, recherche
- **D2 / Mermaid / ECharts** : Rendu de diagrammes d'architecture, diagrammes de flux, graphiques interactifs
- **Partage de session** : Génération de lien de partage en un clic, contrôle d'accès configurable
- **Barre système + raccourci global + démarrage automatique** : Exécution en arrière-plan non intrusive
- **Mise à jour automatique** : Détection automatique des nouvelles versions sur GitHub Releases
- **Support proxy** : Configuration de proxy HTTP et SOCKS5
- **Espace de travail cloud** : Synchronisation de stockage S3 et WebDAV, détection de conflits et synchronisation bidirectionnelle

### Mobile

- Android APK/AAB (arm64-v8a, armeabi-v7a, x86_64)
- iOS IPA (arm64)
- Adaptations spécifiques au mobile : Adaptation de la zone de sécurité, barre de navigation inférieure, navigation par tiroir

---

## Architecture technique

### Stack technique

| Couche               | Technologie                              |
| -------------------- | ---------------------------------------- |
| Framework desktop    | Tauri 2.11                               |
| Framework frontend   | React 19 + TypeScript 6                  |
| Bibliothèque UI      | Ant Design 6 + TailwindCSS 4             |
| Gestion d'état       | Zustand 5                                |
| Routage              | React Router 7                           |
| Éditeur de code      | Monaco Editor                            |
| Terminal             | xterm.js 6                               |
| Éditeur de workflow  | ReactFlow 12                             |
| Graphiques           | D2 + Mermaid + Recharts + ECharts        |
| Défilement virtuel   | @tanstack/react-virtual + react-virtuoso |
| Glisser-déposer      | @dnd-kit                                 |
| Rendu Markdown       | markstream-react + stream-markdown       |
| Internationalisation | i18next + react-i18next                  |
| Outil de build       | Vite 8                                   |
| Tests                | Vitest + Playwright + cargo-nextest      |
| Formatage            | dprint (TS/JSON/Markdown/TOML) + rustfmt |
| Lint                 | ESLint + Oxlint + Clippy + cargo-deny    |

### Architecture backend : Patron d'injection de dépendances Harness

Le backend adopte une architecture workspace Rust comprenant **32 crates**, suivant le **patron d'architecture Harness** :

```
Tous les crates sont découplés via les interfaces trait définies par axagent-harness,
et axagent-runtime assemble et injecte les dépendances à l'exécution.

Direction des dépendances : Implémentations concrètes → harness ← Appelants
```

**harness** est la pierre angulaire de l'architecture — sans logique métier ni implémentation concrète, il ne contient que des définitions de traits, des DTO de données pures, des constantes et un type d'erreur unifié. Tous les autres crates en dépendent, mais il ne dépend lui-même d'aucun autre crate axagent-*.

```
src-tauri/crates/
├── harness/          # Pierre angulaire — interfaces trait, DTO, type d'erreur unifié, contrats DI
│                     #   200+ définitions de traits couvrant : Agent/Provider/Tool/RAG/Stockage/
│                     #   MCP/Plugins/Sécurité/Observabilité/Mémoire/Apprentissage/Navigateur/Messagerie, etc.
│
├── entities/         # Modèles d'entités SeaORM
├── dao/              # Couche d'accès aux données (CRUD)
├── migration/        # Migrations de base de données
│
├── crypto/           # Chiffrement/déchiffrement AES-256-GCM et gestion des clés
├── credential/       # Stockage sécurisé des credentials (clés API, etc.)
├── storage/          # Abstraction de stockage de fichiers (local / S3 / WebDAV), lecture/écriture ZIP
├── cache/            # Couche de cache générique (en mémoire)
├── disk-cache/       # Cache au niveau des fichiers disque
├── search/           # Moteur de recherche (FTS5 + sqlite-vec + embeddings candle)
├── document-parser/  # Extraction de texte de documents (PDF/DOCX/XLSX/PPTX)
├── kit/              # Boîte à outils utilitaire — chemins/encodage/hachage/dates, etc.
│
├── runtime-core/     # Types communs d'exécution, constantes de configuration
├── runtime/          # Orchestration des services d'exécution — assemble les 30+ crates, conteneur d'exécution Harness DI
│                     #   Gère : Sessions/Terminal/Webhooks/Limitation de débit/Permissions/SSRF/Bus d'événements/État
├── rt-workflow/      # Moteur de workflow — orchestration DAG, exécuteurs de nœuds, sérialisation YAML
├── rt-messaging/     # Passerelle de plateforme de messagerie — DingTalk/Feishu/QQ/Slack/WeChat/WhatsApp/Telegram/Discord
├── rt-webhook/       # Serveur Webhook générique et distribution d'événements
├── rt-dashboard/     # Cadre de plugins de tableau de bord
├── rt-theme/         # Moteur de thèmes — logique de commutation sombre/clair
│
├── agent/            # Cœur de l'agent IA — 80+ modules
│                     #   Moteur ReAct/Planification hiérarchique/Recherche approfondie/Vérification de faits/Arbre de pensées/Réflexion/
│                     #   Auto-vérification/Récupération d'erreurs/Optimisation RL/Fine-tuning LoRA/Évaluation/Recommandation d'outils/Test A-B/
│                     #   Coordinateur/Tableau noir/Pipeline de vision/Recherche Web/Recherche académique/Compilation Wiki, etc.
│
├── orchestrator/     # Orchestration d'agents — ordonnancement multi-agents, décomposition DAG, exécution de sous-graphes dynamiques
├── providers/        # Adaptateurs de fournisseurs de modèles — OpenAI/Anthropic/Gemini/Ollama/
│                     #   OpenClaw/Hermes/Génération d'images (DALL-E/Flux)/Realtime/Responses
├── tools/            # Système d'outils — Trait Tool/Registre/Orchestration/Streaming/Sandbox/47+ outils intégrés
├── gateway/          # Passerelle API — serveur axum HTTP/WS, OAuth, limitation de débit, Prometheus
├── mcp/              # Protocole MCP — stdio + HTTP Streamable, basé sur rmcp
├── trajectory/       # Système d'apprentissage — Mémoire/Évolution des compétences/Profil utilisateur/Intégration onirique
├── plugins/          # Système de plugins — Compatible OpenClaw, installation de packages npm, marketplace
├── telemetry/        # Observabilité — OpenTelemetry, journalisation structurée, métriques d'exécution
├── prompt-guard/     # Protection contre l'injection de prompts — pipeline de détection multi-niveaux L1-L4
├── npm/              # Client de registre npm
└── schema-gen/       # Outil de génération de schéma de base de données
```

### Architecture frontend

```
src/
├── pages/            # 22 pages
│   ├── ChatPage          # Interface de chat principale
│   ├── WorkflowPage      # Éditeur de workflow
│   ├── GatewayPage       # Gestion de la passerelle API
│   ├── KnowledgeHubPage  # Gestion de la base de connaissances
│   ├── MemoryPage        # Gestion de la mémoire
│   ├── SkillsPage        # Marketplace de compétences
│   ├── SettingsPage      # Panneau de paramètres
│   ├── DashboardPage     # Tableau de bord de données
│   ├── TerminalPage      # Terminal
│   ├── FilesPage         # Gestion de fichiers
│   ├── GatewayLinkPage   # Gestion des liens externes
│   ├── LinkPage          # Liens d'intégration
│   ├── WikiEditorPage    # Éditeur Wiki
│   ├── WikiEditPage      # Édition Wiki
│   ├── WikiGraphPage     # Graphe de connaissances Wiki
│   ├── FineTunePage      # Fine-tuning LoRA
│   ├── PersonaPage       # Gestion de personas
│   ├── QuickBarPage      # Barre rapide
│   ├── IngestPage        # Ingestion de documents
│   ├── WorkflowMarketplace # Marketplace de workflows
│   ├── DynamicUIManagerPage # Gestion d'UI dynamique
│   └── DynamicPageViewer    # Visionneuse de pages dynamiques
│
├── components/       # 24 modules, 200+ composants
│   ├── chat/         # Interface de chat (flux de messages/entrée/pièces jointes/appels d'outils/artefacts/blocs de réflexion, etc.)
│   ├── workflow/     # Éditeur de workflow (nœuds/arêtes/panneaux/modèles/assistance IA)
│   ├── gateway/      # UI de gestion de la passerelle API
│   ├── settings/     # Panneau de paramètres (40+ sous-composants)
│   ├── skill/        # Éditeur et moteur de rendu de compétences
│   ├── benchmark/    # Panneau de benchmarks
│   ├── decomposition/# Décomposition de compétences et génération d'outils
│   ├── devtools/     # Chronologie Trace/Span
│   ├── layout/       # Mise en page (barre de titre/barre latérale/palette de commandes)
│   └── ...
│
├── stores/           # 62 stores Zustand
│   ├── domain/       # État métier principal
│   ├── feature/      # État des modules fonctionnels (44)
│   └── devtools/     # État des outils de développement
│
├── hooks/            # Hooks React
├── lib/              # Fonctions utilitaires + Web Workers
├── types/            # Définitions de types TypeScript
├── sdk/              # SDK d'intégration externe
└── i18n/             # Traductions en 11 langues (zh-CN/zh-TW/en-US/ja/ko/fr/de/es/ru/hi/ar)
```

---

## Répertoires de données

```
~/.axagent/                    # Configuration de l'application
├── axagent.db                 # Base de données SQLite principale (SeaORM)
├── master.key                 # Clé maîtresse AES-256
├── vector_db/                 # Index vectoriel sqlite-vec
└── ssl/                       # Certificats SSL auto-signés

~/Documents/axagent/          # Fichiers utilisateur
├── images/                   # Pièces jointes images
├── files/                    # Pièces jointes fichiers
└── backups/                  # Sauvegardes automatiques
```

---

## Démarrage rapide

### Prérequis

- [Node.js](https://nodejs.org/) 20+
- [Rust](https://www.rust-lang.org/) 1.75+, edition 2024
- [npm](https://www.npmjs.com/) 10+
- Windows : [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) (MSVC + Windows SDK)
- macOS : Xcode Command Line Tools
- Linux : `build-essential` + `libwebkit2gtk-4.1-dev` + `libssl-dev`

### Build

```bash
git clone https://github.com/polite0803/AxAgent.git
cd AxAgent
npm install
npm run tauri dev      # Mode développement
npm run tauri build    # Build de production
```

Les artefacts de build se trouvent dans `src-tauri/target/release/`.

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
npm run format

# Vérification CI complète
npm run ci:check
```

---

## Support des plateformes

| Plateforme | Architecture                               |
| ---------- | ------------------------------------------ |
| Windows    | x86_64, ARM64                              |
| macOS      | Apple Silicon (arm64), Intel (x86_64)      |
| Linux      | x86_64, ARM64                              |
| Android    | arm64-v8a, armeabi-v7a, x86_64 (émulateur) |
| iOS        | arm64                                      |

---

## Licence

Ce projet est publié en open source sous la licence [AGPL-3.0-only](LICENSE).

---

## Remerciements

AxAgent est construit sur de nombreux excellents projets open source, y compris mais sans s'y limiter :

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
