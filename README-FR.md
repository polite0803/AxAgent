[English](./README-EN.md) | [简体中文](./README.md) | [繁體中文](./README-ZH-TW.md) | [日本語](./README-JA.md) | [한국어](./README-KO.md) | **Français** | [Deutsch](./README-DE.md) | [Español](./README-ES.md) | [Русский](./README-RU.md) | [हिन्दी](./README-HI.md) | [العربية](./README-AR.md)

[![AxAgent](https://github.com/polite0803/AxAgent/blob/main/src/assets/image/logo.png?raw=true)](https://github.com/polite0803/AxAgent)

<p align="center">
  <a href="https://www.producthunt.com/products/axagent?embed=true&amp;amp;utm_source=badge-featured&amp;amp;&amp;#10;&amp;amp;amp;;utm_medium=badge&amp;amp;#10&amp;amp;;utm_campaign=badge-axagent" target="_blank" rel="noopener noreferrer"><img alt="AxAgent - Lightweight, high-perf cross-platform AI desktop client | Product Hunt" width="250" height="54" src="https://api.producthunt.com/widgets/embed-image/v1/featured.svg?post_id=1118403&amp;theme=light&amp;t=1775627359538"></a>
</p>

<p align="center">
  <strong>Client desktop IA multiplateforme | Collaboration multi-agents | Local d'abord</strong>
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

## Qu'est-ce qu'AxAgent ?

AxAgent est une application desktop IA multiplateforme complète, intégrant des capacités d'agents IA avancées et des outils de développement riches. Elle prend en charge plusieurs fournisseurs de modèles, l'exécution autonome de pipelines, l'orchestration visuelle de flux de travail, la gestion locale des connaissances et une passerelle API intégrée.

---

## Aperçu des captures d'écran

| Conversation et sélection de modèle | Tableau de bord multi-agents |
|:---:|:---:|
| ![](.github/images/s1-0412.png) | ![](.github/images/s5-0412.png) |

| Base de connaissances RAG | Mémoire et contexte |
|:---:|:---:|
| ![](.github/images/s3-0412.png) | ![](.github/images/s4-0412.png) |

| Éditeur de flux de travail | Passerelle API |
|:---:|:---:|
| ![](.github/images/s9-0412.png) | ![](.github/images/s10-0412.png) |

---

## Fonctionnalités principales

### 🤖 Prise en charge des modèles IA

- **Support multi-fournisseurs** — Intégration native d'OpenAI, Anthropic Claude, Google Gemini, Ollama, OpenClaw, Hermes et de toutes les API compatibles OpenAI
- **Rotation multi-clés** — Configurez plusieurs clés API par fournisseur avec rotation automatique pour distribuer la pression des limites de débit
- **Support des modèles locaux** — Prise en charge complète des modèles locaux Ollama, incluant la gestion des fichiers GGUF/GGML
- **Gestion des modèles** — Récupération des listes de modèles distants, personnalisation des paramètres (température, tokens max, top-p, etc.)
- **Sortie en streaming** — Rendu en temps réel token par token avec blocs de réflexion repliables (pensée étendue Claude)
- **Comparaison multi-modèles** — Posez la même question à plusieurs modèles simultanément avec comparaison côte à côte
- **Appel de fonctions** — Appels de fonctions structurés sur tous les fournisseurs pris en charge
- **API Responses OpenAI** — Prise en charge du transport au format OpenAI Responses
- **API Realtime** — Push d'événements WebSocket compatible avec l'API Realtime OpenAI

### 🔐 Système d'agents IA

Le système d'agents est construit sur une architecture sophistiquée avec les caractéristiques suivantes :

- **Moteur de raisonnement ReAct** — Fusion du raisonnement et de l'action, avec auto-vérification intégrée pour une exécution fiable des tâches
- **Planificateur hiérarchique** — Décomposition des tâches complexes en plans structurés avec phases et dépendances
- **Décomposeur de tâches** — Décomposition automatique des tâches complexes en sous-tâches exécutables
- **Recherche approfondie** — Orchestration de recherches multi-sources, suivi des citations et évaluation de la crédibilité
- **Vérification des faits** — Vérification des faits pilotée par l'IA et classification des sources
- **Orchestration de recherche** — Coordination de multiples fournisseurs de recherche, avec planification et synthèse des résultats
- **Recherche académique** — Recherche de littérature académique et analyse des citations
- **Contrôle informatique** — Clics de souris, saisie au clavier, défilement d'écran contrôlés par l'IA, avec analyse par modèle visuel
- **Perception d'écran** — Capture d'écran et analyse par modèle visuel pour l'identification des éléments UI
- **Trois niveaux de permissions** — Par défaut (approbation requise), Accepter les modifications (approbation automatique), Accès complet (sans invite)
- **Isolation sandbox** — Les opérations de l'agent sont strictement limitées au répertoire de travail spécifié
- **Panneau d'approbation des outils** — Affichage en temps réel des demandes d'appel d'outils avec approbation individuelle
- **Suivi des coûts** — Affichage en temps réel de l'utilisation des tokens et des statistiques de coûts par session
- **Pause/Reprise** — Suspendez l'exécution de l'agent à tout moment et reprenez plus tard
- **Système de points de contrôle** — Points de contrôle persistants pour la récupération après crash et la reconnexion de session
- **Moteur de récupération d'erreurs** — Classification automatique des erreurs, analyse des causes racines et exécution de stratégies de récupération
- **Détection de boucles** — Détection et interruption automatiques des comportements en boucle dans le raisonnement de l'agent
- **Chaîne de pensée** — Visualisation du raisonnement décisionnel de l'agent, décomposition étape par étape
- **Mode proactif** — L'agent peut proposer des suggestions et exécuter des actions de manière proactive
- **Gestion des objectifs** — Maintien et suivi des objectifs d'exécution et du contexte de l'agent

### 👥 Collaboration multi-agents

- **Coordination sous-agent** — Architecture maître-esclave avec support de multiples agents collaboratifs
- **Exécution parallèle** — Traitement parallèle par plusieurs agents avec planification sensible aux dépendances
- **Débat contradictoire** — Tours de débat Pro/Con avec notation de la force des arguments et suivi des réfutations
- **Rôles d'agents** — Rôles prédéfinis (chercheur, planificateur, développeur, réviseur, synthétiseur) pour la collaboration en équipe
- **Orchestrateur d'agents** — Routage centralisé des messages et gestion de l'état pour les équipes multi-agents
- **Graphe de communication** — Visualisation des interactions et des flux de messages entre agents
- **Cluster Swarm** — Cluster d'agents multi-processus avec synchronisation des permissions et reconnexion automatique
- **Système Buddy** — Agents partenaires configurables avec définition d'espèces et d'attributs
- **Mémoire partagée** — Espace mémoire partagé entre agents avec statistiques et requêtes
- **Cron d'équipe** — Planification de tâches cron au niveau de l'équipe

### ⭐ Système de compétences

- **Marché des compétences** — Marché intégré pour parcourir et installer des compétences contribuées par la communauté
- **Création de compétences** — Création automatique de compétences à partir de propositions, avec éditeur Markdown
- **Évolution des compétences** — Analyse et amélioration automatiques pilotées par l'IA des compétences existantes basées sur les retours d'exécution
- **Correspondance des compétences** — Correspondance sémantique, recommandation de compétences pertinentes au contexte de conversation
- **Décomposition des compétences** — Décomposition automatique des tâches complexes en compétences atomiques exécutables (assistée par LLM/multi-tours/validation par flux de travail)
- **Outils générés** — Génération et enregistrement automatiques par l'IA de nouveaux outils pour étendre les capacités de l'agent
- **Hub de compétences** — Interface centralisée de découverte et de gestion de la configuration des compétences
- **Client du hub de compétences** — Intégration avec un hub de compétences distant, avec partage communautaire
- **Vérification des dépendances de compétences** — Détection automatique des dépendances de compétences et de la disponibilité des outils
- **Conteneur sandbox de compétences** — Exécution sécurisée des compétences dans un environnement isolé

### 🔄 Système de flux de travail

Le moteur de flux de travail implémente un système d'orchestration de tâches basé sur les DAG :

- **Éditeur de flux de travail visuel** — Concepteur de flux de travail par glisser-déposer avec connexion et configuration de nœuds
- **Types de nœuds riches** — 15 types de nœuds : déclencheur, agent, LLM, condition, parallèle, boucle, fusion, délai, outil, code, sous-flux de travail, recherche vectorielle, analyse de document, validation, fin
- **Modèles de flux de travail** — Préréglages intégrés : revue de code, correction de bugs, documentation, tests, refactoring, exploration, performance, sécurité, développement de fonctionnalités
- **Exécution DAG** — Tri topologique par algorithme de Kahn, avec détection de cycles
- **Planification parallèle** — Exécution en pipeline, les étapes rapides n'attendent pas les lentes
- **Stratégie de retry** — Backoff exponentiel, nombre maximal de tentatives configurable par étape
- **Achèvement partiel** — Les étapes échouées ne bloquent pas les étapes descendantes indépendantes
- **Gestion des versions** — Contrôle de version des modèles de flux de travail avec retour en arrière
- **Historique d'exécution** — Enregistrement détaillé avec suivi d'état et débogage
- **Assistance IA** — Conception de flux de travail assistée par IA, recommandation de nœuds et optimisation des prompts d'agents
- **Vérification sémantique** — Validation sémantique des flux de travail, détection des problèmes potentiels
- **Import n8n** — Prise en charge de l'import de flux de travail depuis un répertoire n8n
- **Panneau de débogage** — Débogage en temps réel et visualisation de l'état pendant l'exécution du flux de travail

### 📚 Connaissances et mémoire

- **Base de connaissances (RAG)** — Support multi-bases de connaissances, téléchargement de documents, analyse automatique, découpage et indexation vectorielle
- **Recherche hybride** — Combinaison de recherche par similarité vectorielle et de classement BM25 en texte intégral
- **Reranking** — Reranking par cross-encoder pour améliorer la précision de récupération
- **Pipeline de rappel à trois niveaux** — Mécanisme de rappel multi-niveau avec index AST + recherche vectorielle + FTS5
- **Graphe de connaissances** — Visualisation des relations entité-connaissance (entités, attributs, relations, flux, interfaces)
- **Système Wiki** — Compilateur et validateur LLM Wiki, avec visualisation du graphe de connaissances et synchronisation incrémentale
- **Notes Wiki** — Système de notes avec liens bidirectionnels, vue en graphe et synchronisation automatique des liens
- **Système de mémoire** — Mémoire multi-espaces de noms, avec saisie manuelle ou extraction automatique par l'IA
- **Mémoire en boucle fermée** — Intégration des fournisseurs de mémoire persistante Honcho et Mem0
- **Recherche plein texte FTS5** — Recherche rapide dans les conversations, fichiers et mémoires
- **Recherche de sessions** — Recherche avancée dans toutes les sessions de conversation
- **Gestion du contexte** — Attachement flexible de fichiers, résultats de recherche, passages de connaissances, mémoires, sorties d'outils
- **Analyse de documents** — Analyse automatique et extraction de contenu de documents multi-formats
- **Indexation incrémentale** — Mise à jour incrémentale de l'index lors des modifications de fichiers

### 🌐 Passerelle API

- **Serveur API local** — Serveur intégré compatible OpenAI, Claude et Gemini
- **Liens externes** — Intégration en un clic avec Claude CLI, OpenCode, synchronisation automatique des clés API et des modèles
- **Gestion des clés** — Génération, révocation, activation/désactivation des clés d'accès avec descriptions
- **Analyse d'utilisation** — Volume de requêtes et utilisation des tokens par clé, fournisseur et date
- **Support SSL/TLS** — Certificats auto-signés intégrés, support de certificats personnalisés
- **Journaux de requêtes** — Enregistrement complet de toutes les requêtes et réponses API
- **Modèles de configuration** — Modèles pré-construits pour Claude, Codex, OpenCode, Gemini
- **API Realtime** — Push d'événements WebSocket compatible avec l'API Realtime OpenAI
- **Intégration de plateforme** — Support de DingTalk, Feishu, QQ, Slack, WeChat, WhatsApp, Telegram, Discord
- **Diagnostics de passerelle** — Diagnostics de connexion et gestion des stratégies de programme
- **Limiteur de débit** — Limitation du taux de requêtes API et contrôle du flux
- **File persistante** — Gestion de file de requêtes persistante

### 🔧 Outils et extensions

- **Protocole MCP** — Implémentation complète du Model Context Protocol, support des transports stdio et HTTP/WebSocket
- **Authentification OAuth** — Support du flux OAuth pour les serveurs MCP
- **Démarrage automatique MCP** — Démarrage automatique et gestion du cycle de vie des serveurs MCP
- **Pont d'outils MCP** — Pont entre les outils MCP et le système d'outils de l'agent
- **Système de plugins** — Architecture de plugins à trois niveaux (intégré/lié/externe), avec enregistrement d'outils, hooks et gestion du cycle de vie
- **Outils intégrés** — Opérations de fichiers complètes (lecture/écriture/édition), exécution de code, recherche (Grep/Glob), Bash, recherche web, extraction web, gestion de plans, planification Cron, REPL, LSP, gestion de contexte, contrôle informatique, envoi de messages, liste de tâches, etc.
- **Système de permissions d'outils** — Classification des permissions d'outils, gestion des règles et suivi de l'utilisation
- **Sécurité Bash** — Analyse de commandes, validation de chemins et contrôle de sécurité sandbox
- **Client LSP** — Protocole Language Server intégré, complétion de code et diagnostics
- **Index AST** — Analyse et indexation AST des fichiers de code
- **Backend terminal** — Support des connexions terminal locales, Docker et SSH
- **Automatisation de navigateur** — Contrôle de navigateur via CDP (navigation, captures d'écran, clics, remplissage, extraction de texte, etc.)
- **Automatisation UI** — Identification et contrôle d'éléments UI multiplateforme
- **Outils Git** — Opérations Git avec détection de branches et sensibilité aux conflits
- **Recommandation d'outils** — Moteur de recommandation intelligent d'outils basé sur le contexte
- **Orchestration d'outils** — Coordination et exécution multi-outils avec sortie en streaming
- **Statistiques d'outils** — Statistiques de fréquence d'utilisation et de performance des outils

### 📊 Rendu de contenu

- **Rendu Markdown** — Support complet de la coloration syntaxique, formules mathématiques LaTeX, tableaux, listes de tâches
- **Éditeur de code Monaco** — Éditeur intégré avec coloration syntaxique, copie, aperçu diff
- **Rendu de diagrammes** — Diagrammes de flux Mermaid, diagrammes d'architecture D2, graphiques interactifs ECharts
- **Panneau d'artefacts** — Extraits de code, brouillons HTML, composants React, notes Markdown, avec aperçu en temps réel
- **Quatre modes d'aperçu** — Code (éditeur), Split (côte à côte), Aperçu (rendu uniquement), Aperçu de composant React
- **Inspecteur de session** — Vue arborescente de la structure de session, navigation rapide
- **Panneau de citations** — Suivi et affichage des citations sources avec notation de crédibilité
- **Rendu d'infographies** — Prise en charge de la visualisation d'infographies

### 🛡️ Données et sécurité

- **Chiffrement AES-256** — Clés API et données sensibles chiffrées avec AES-256-GCM
- **Stockage isolé** — État de l'application dans `~/.axagent/`, fichiers utilisateur dans `~/Documents/axagent/`
- **Sauvegarde automatique** — Sauvegardes planifiées vers un répertoire local ou un stockage WebDAV
- **Restauration de sauvegarde** — Restauration en un clic depuis les sauvegardes historiques
- **Options d'export** — Captures PNG, Markdown, texte brut, JSON
- **Gestion du stockage** — Affichage visuel de l'utilisation du disque et outils de nettoyage
- **Autorisation de fichiers** — Gestion des autorisations et révocation de l'accès aux fichiers
- **Audit des opérations** — Journal d'audit des opérations critiques

### 🖥️ Expérience bureau

- **Moteur de thèmes** — Thèmes sombre/clair, suivi du système ou préférence manuelle
- **Langue d'interface** — 11 langues : chinois simplifié, chinois traditionnel, anglais, japonais, coréen, français, allemand, espagnol, russe, hindi, arabe
- **Barre d'état système** — Minimisation dans la barre d'état sans interruption des services en arrière-plan
- **Fenêtre toujours au premier plan** — Fenêtre épinglée au-dessus des autres fenêtres
- **Raccourcis globaux** — Raccourcis clavier personnalisables pour appeler la fenêtre principale
- **QuickBar** — Barre flottante d'accès rapide, invocation en un clic
- **Démarrage automatique** — Lancement optionnel au démarrage du système
- **Support proxy** — Configuration de proxy HTTP et SOCKS5
- **Mise à jour automatique** — Vérification automatique des versions, notification de mise à jour
- **Palette de commandes** — `Cmd/Ctrl+K` pour un accès rapide aux commandes
- **Assistant d'intégration** — Guide interactif de première utilisation et détection d'Ollama
- **Centre de notifications** — Gestion unifiée des notifications dans l'application

### 🔬 Fonctionnalités avancées

- **Recherche approfondie** — Recherche multi-sources, suivi des citations, évaluation de crédibilité et synthèse de contenu
- **Vérification des faits** — Vérification des faits pilotée par l'IA et classification des sources
- **Planificateur Cron** — Planification de tâches automatisées avec modèles quotidien/hebdomadaire/mensuel et expressions cron personnalisées
- **Système Webhook** — Abonnement aux événements, notifications de complétion d'outils, erreurs d'agents, fin de session
- **Profil utilisateur** — Apprentissage automatique du style de code, conventions de nommage, indentation, style de commentaires, préférences de communication
- **Optimiseur RL** — Optimisation par apprentissage par renforcement de la sélection d'outils et des stratégies de tâches
- **Ajustement LoRA** — Adaptation de modèle personnalisée avec ajustement LoRA local
- **Suggestions proactives** — Invites contextuelles basées sur le contenu de conversation et les modèles utilisateur
- **Prédiction de contexte** — Prédiction des prochaines actions de l'utilisateur et préchargement des ressources pertinentes
- **Intégration onirique** — Intégration automatique en arrière-plan des mémoires et patterns, optimisation des connaissances à long terme
- **Récupération d'erreurs** — Classification automatique des erreurs, analyse des causes racines et suggestions de récupération
- **Outils de développement** — Trace, Span, visualisation de timeline pour le débogage et l'analyse de performance
- **Système de benchmark** — Évaluation des performances SWE-bench / Terminal-bench avec scorecards
- **Transfert de style** — Application des préférences de style de code apprises au code généré
- **Plugins de tableau de bord** — Tableau de bord extensible avec panneaux et widgets personnalisés
- **Collaboration et partage** — Collaboration temps réel CRDT et partage de session en un clic
- **Extension de navigateur** — Extension de navigateur Wiki Clipper pour le clipping rapide de pages web vers le Wiki LLM
- **SDK Python** — SDK Python pour l'intégration avec AxAgent
- **Routeur intelligent** — Routage et classification intelligents des requêtes
- **Cache sémantique** — Cache de réponses basé sur la sémantique, réduction du calcul redondant
- **Compression de contexte** — Compression automatique des contextes longs, optimisation de l'utilisation des tokens
- **Traitement par lots de messages** — Envoi et optimisation par lots des messages
- **Pool de connexions** — Gestion du pool de connexions base de données et API
- **Feature flags** — Système de feature flags configurable
- **Moteur de politiques** — Gestion centralisée des politiques de permissions et d'opérations
- **Gouverneur de ressources** — Limitation et gouvernance de l'utilisation des ressources par les agents
- **Transfert LAN** — Capacité de transfert de fichiers en réseau local

---

## Architecture technique

### Pile technologique

| Couche | Technologie |
|--------|-------------|
| **Framework** | Tauri 2 + React 19 + TypeScript 6 |
| **UI** | Ant Design 6 + TailwindCSS 4 |
| **Gestion d'état** | Zustand 5 |
| **Routage** | React Router 7 |
| **i18n** | i18next + react-i18next |
| **Backend** | Rust + SeaORM 2 + SQLite |
| **Base vectorielle** | sqlite-vec |
| **Éditeur de code** | Monaco Editor |
| **Diagrammes** | Mermaid + D2 + ECharts (CDN) |
| **Terminal** | xterm.js 6 |
| **Flux de travail** | ReactFlow 11 |
| **Build** | Vite 8 + npm |

### Architecture Backend Rust

Le backend est organisé comme un workspace Rust avec 10 crates spécialisées :

```
src-tauri/crates/
├── agent/         # Noyau de l'agent IA
│   ├── react_engine.rs          # Moteur de raisonnement ReAct
│   ├── coordinator.rs           # Coordination des agents
│   ├── hierarchical_planner.rs  # Décomposition des tâches
│   ├── task_decomposer.rs       # Décomposition des sous-tâches
│   ├── self_verifier.rs         # Vérification des sorties
│   ├── verification_agent.rs    # Agent de vérification
│   ├── error_recovery_engine.rs # Moteur de récupération d'erreurs
│   ├── error_classifier.rs      # Classification des erreurs
│   ├── recovery_strategies.rs   # Stratégies de récupération
│   ├── loop_detector.rs         # Détection de boucles
│   ├── vision_pipeline.rs       # Perception d'écran
│   ├── deep_research.rs         # Recherche approfondie
│   ├── fact_checker.rs          # Vérification des faits
│   ├── research_agent.rs        # Agent de recherche
│   ├── search_planner.rs        # Planification de recherche
│   ├── search_orchestrator.rs   # Orchestration de recherche
│   ├── academic_search.rs       # Recherche académique
│   ├── source_validator.rs      # Validation des sources
│   ├── source_classifier.rs     # Classification des sources
│   ├── credibility_evaluator.rs # Évaluation de crédibilité
│   ├── citation_tracker.rs      # Suivi des citations
│   ├── content_synthesizer.rs   # Synthèse de contenu
│   ├── outline_builder.rs       # Construction de plans
│   ├── reference_builder.rs     # Construction de références
│   ├── proactive_mode.rs        # Mode proactif
│   ├── purpose_manager.rs       # Gestion des objectifs
│   ├── graph_insights.rs        # Insights de graphe
│   ├── insight_generator.rs     # Génération d'insights
│   ├── schema_manager.rs        # Gestion de schéma
│   ├── ingest_pipeline.rs       # Pipeline d'ingestion de données
│   ├── session_manager.rs       # Gestion des sessions
│   ├── health_checker.rs        # Vérification de santé
│   ├── metrics.rs               # Collecte de métriques
│   ├── evaluator/               # Évaluation de benchmarks
│   ├── fine_tune/               # Ajustement LoRA
│   ├── rl_optimizer/            # Optimisation des stratégies RL
│   └── tool_recommender/        # Moteur de recommandation d'outils
│
├── core/          # Utilitaires principaux
│   ├── db.rs                   # Base de données SeaORM
│   ├── vector_store.rs         # Intégration sqlite-vec
│   ├── rag.rs                  # Couche d'abstraction RAG
│   ├── hybrid_search.rs        # Recherche vectorielle + FTS5
│   ├── recall_pipeline.rs      # Pipeline de rappel à trois niveaux
│   ├── crypto.rs               # Chiffrement AES-256
│   ├── mcp_client.rs           # Client protocole MCP
│   ├── browser_automation.rs   # Automatisation de navigateur
│   ├── computer_control.rs     # Contrôle informatique
│   ├── screen_vision.rs        # Vision d'écran
│   ├── screen_capture.rs       # Capture d'écran
│   ├── ui_automation.rs        # Automatisation UI
│   ├── ast_index.rs            # Index AST
│   ├── incremental_indexer.rs  # Indexation incrémentale
│   ├── document_parser.rs      # Analyse de documents
│   ├── markdown_parser.rs      # Analyse Markdown
│   ├── text_chunker.rs         # Découpage de texte
│   ├── token_counter.rs        # Comptage de tokens
│   ├── token_budget.rs         # Budget de tokens
│   ├── file_index.rs           # Index de fichiers
│   ├── file_authorizer.rs      # Autorisation de fichiers
│   ├── file_store.rs           # Stockage de fichiers
│   ├── cache.rs                # Gestion du cache
│   ├── disk_cache.rs           # Cache disque
│   ├── cache_persister.rs      # Persistance du cache
│   ├── cache_snapshot.rs       # Instantané de cache
│   ├── vector_cache.rs         # Cache vectoriel
│   ├── marketplace_service.rs  # Service de marché
│   ├── marketplace.rs          # Abstraction du marché
│   ├── operation_audit.rs      # Audit des opérations
│   ├── unified_config.rs       # Configuration unifiée
│   ├── platform_config.rs      # Configuration plateforme
│   ├── command_validator.rs    # Validation de commandes
│   ├── shell_parser.rs         # Analyse Shell
│   ├── output_processor.rs     # Traitement des sorties
│   ├── storage_inventory.rs    # Inventaire de stockage
│   ├── storage_migration.rs    # Migration de stockage
│   ├── storage_paths.rs        # Chemins de stockage
│   ├── s3_backup.rs            # Sauvegarde S3
│   ├── webdav.rs               # Synchronisation WebDAV
│   ├── git_tools.rs            # Outils Git
│   ├── sandbox_runner.rs       # Exécuteur sandbox
│   ├── search.rs               # Abstraction de recherche
│   ├── reranker.rs             # Reranking
│   ├── model_knowledge.rs      # Connaissance des modèles
│   ├── prompt_template.rs      # Modèles de prompts
│   ├── preset_templates.rs     # Modèles prédéfinis
│   ├── workflow_types.rs       # Types de flux de travail
│   ├── workflow_version.rs     # Version de flux de travail
│   ├── path_vars.rs            # Variables de chemin
│   ├── entity/                 # Entités SeaORM (40+ tables)
│   └── repo/                   # Dépôts de données (30+ dépôts)
│
├── gateway/       # Passerelle API
│   ├── server.rs               # Serveur HTTP
│   ├── handlers.rs             # Gestionnaires API
│   ├── routes.rs               # Définition des routes
│   ├── auth.rs                 # Authentification
│   ├── middleware.rs           # Middleware
│   ├── metrics.rs              # Collecte de métriques
│   ├── native.rs               # Intégration native
│   ├── marketplace_handlers.rs # Interface du marché
│   └── realtime.rs             # Support WebSocket
│
├── plugins/       # Système de plugins
│   ├── hooks.rs                # Exécuteur de hooks
│   ├── agent_provider.rs       # Fournisseur d'agents
│   ├── test_isolation.rs       # Isolation de test
│   └── lib.rs                  # Registre de plugins et cycle de vie
│
├── providers/     # Adaptateurs de modèles
│   ├── adapter.rs              # Interface d'adaptateur
│   ├── registry.rs             # Registre des fournisseurs
│   ├── openai.rs               # API OpenAI
│   ├── openai_responses.rs     # API Responses OpenAI
│   ├── anthropic.rs            # API Claude
│   ├── gemini.rs               # API Gemini
│   ├── ollama.rs               # Ollama local
│   ├── openclaw.rs             # OpenClaw
│   ├── hermes.rs               # Hermes
│   ├── image_gen.rs            # Génération d'images
│   ├── realtime_client.rs      # Client API Realtime
│   └── transport/              # Couche de transport (Chat Completions / Responses / Anthropic)
│
├── runtime/       # Services d'exécution
│   ├── session.rs              # Gestion des sessions
│   ├── workflow_engine.rs      # Orchestration DAG
│   ├── work_engine/            # Moteur de travail (exécuteurs de nœuds + planificateur + couche de cache)
│   ├── mcp.rs                  # Serveur MCP
│   ├── mcp_client.rs           # Client MCP
│   ├── mcp_server.rs           # Implémentation du serveur MCP
│   ├── mcp_stdio.rs            # Transport MCP stdio
│   ├── mcp_autostart.rs        # Démarrage automatique MCP
│   ├── mcp_lifecycle_hardened.rs # Gestion du cycle de vie MCP
│   ├── mcp_tool_bridge.rs      # Pont d'outils MCP
│   ├── cron/                   # Planification de tâches
│   ├── terminal/               # Backend terminal (local/Docker/SSH)
│   ├── benchmarks/             # SWE-bench / Terminal-bench
│   ├── collaboration/          # Collaboration CRDT et partage de session
│   ├── tool_generator/         # Génération d'outils IA
│   ├── message_gateway/        # Intégration de plateforme (DingTalk/Feishu/QQ/Slack/WeChat/WhatsApp/Telegram/Discord)
│   ├── buddy/                  # Système Buddy (espèces/attributs/gestionnaire)
│   ├── swarm/                  # Cluster Swarm (backend de processus/synchronisation de permissions/reconnexion)
│   ├── tasks/                  # Tâches en arrière-plan (rêves/agents distants/coéquipiers in-process)
│   ├── adversarial_debate.rs   # Débat contradictoire
│   ├── agent_orchestrator.rs   # Orchestration multi-agents
│   ├── agent_roles.rs          # Rôles d'agents
│   ├── webhook_dispatcher.rs   # Distribution de webhooks
│   ├── webhook_server.rs       # Serveur de webhooks
│   ├── session_search.rs       # Recherche de sessions
│   ├── dashboard_plugin.rs     # Plugin de tableau de bord
│   ├── dashboard_registry.rs   # Registre de tableau de bord
│   ├── permissions.rs          # Gestion des permissions
│   ├── permission_enforcer.rs  # Application des permissions
│   ├── policy_engine.rs        # Moteur de politiques
│   ├── trust_resolver.rs       # Résolution de confiance
│   ├── resource_governor.rs    # Gouverneur de ressources
│   ├── green_contract.rs       # Contrat vert
│   ├── feature_flags.rs        # Feature flags
│   ├── module_switch.rs        # Commutateur de modules
│   ├── mode_selector.rs        # Sélecteur de mode
│   ├── config.rs               # Configuration d'exécution
│   ├── config_validate.rs      # Validation de configuration
│   ├── prompt.rs               # Gestion des prompts
│   ├── prompt_cache.rs         # Cache de prompts
│   ├── compact.rs              # Compression de contexte
│   ├── summary_compression.rs  # Compression de résumé
│   ├── compact_thresholds.rs   # Seuils de compression
│   ├── compact_warning.rs      # Avertissement de compression
│   ├── reactive_compact.rs     # Compression réactive
│   ├── session_memory_compact.rs # Compression de mémoire de session
│   ├── message_importance.rs   # Évaluation de l'importance des messages
│   ├── message_batching.rs     # Traitement par lots des messages
│   ├── rate_limiter.rs         # Limiteur de débit
│   ├── connection_pool.rs      # Pool de connexions
│   ├── persistent_queue.rs     # File persistante
│   ├── persistent_queue_manager.rs # Gestionnaire de file
│   ├── health_check.rs         # Vérification de santé
│   ├── cache_guard.rs          # Garde de cache
│   ├── checkpoint.rs           # Point de contrôle
│   ├── branch_lock.rs          # Verrou de branche
│   ├── stale_base.rs           # Détection de base périmée
│   ├── watch_patterns.rs       # Patterns de surveillance
│   ├── lan_transfer.rs         # Transfert LAN
│   ├── tls_config.rs           # Configuration TLS
│   ├── sse.rs                  # Flux d'événements SSE
│   ├── api_server.rs           # Serveur API
│   ├── gateway_auth.rs         # Authentification de passerelle
│   ├── gateway_metrics.rs      # Métriques de passerelle
│   ├── bash.rs                 # Exécution Bash
│   ├── bash_validation.rs      # Validation Bash
│   ├── shell_hooks.rs          # Hooks Shell
│   ├── shell_completer.rs      # Complétion Shell
│   ├── terminal_analyzer.rs    # Analyse de terminal
│   ├── git_context.rs          # Contexte Git
│   ├── git_tools.rs            # Outils Git
│   ├── file_ops.rs             # Opérations de fichiers
│   ├── hooks.rs                # Gestion des hooks
│   ├── hook_chain.rs           # Chaîne de hooks
│   ├── hook_config.rs          # Configuration de hooks
│   ├── plugin_hooks.rs         # Hooks de plugins
│   ├── plugin_lifecycle.rs     # Cycle de vie des plugins
│   ├── profile.rs              # Profil
│   ├── profile_manager.rs      # Gestionnaire de profils
│   ├── oauth.rs                # Authentification OAuth
│   ├── usage.rs                # Statistiques d'utilisation
│   ├── bootstrap.rs            # Amorçage
│   ├── worker_boot.rs          # Démarrage du worker
│   ├── fork_bridge.rs          # Pont de fork
│   ├── task_packet.rs          # Paquet de tâches
│   ├── task_router.rs          # Routeur de tâches
│   ├── task_registry.rs        # Registre de tâches
│   ├── transform_pipeline.rs   # Pipeline de transformation
│   ├── transport_handlers.rs   # Gestionnaires de transport
│   ├── general_engine.rs       # Moteur général
│   ├── engine_bridge.rs        # Pont de moteur
│   ├── conversation.rs         # Gestion de conversation
│   ├── session_control.rs      # Contrôle de session
│   ├── shared_memory.rs        # Mémoire partagée
│   ├── validation_executor.rs  # Exécuteur de validation
│   ├── recovery_recipes.rs     # Recettes de récupération
│   ├── error_recovery.rs       # Récupération d'erreurs
│   ├── theme_engine.rs         # Moteur de thèmes
│   ├── token_budget_predictor.rs # Prédiction de budget de tokens
│   ├── team_cron_registry.rs   # Registre Cron d'équipe
│   ├── module_dream.rs         # Module onirique
│   ├── json.rs                 # Utilitaires JSON
│   └── lane_events.rs          # Événements Lane
│
├── telemetry/     # Télémétrie et traçage
│   ├── tracer.rs              # Traçage distribué
│   ├── metrics.rs             # Collecte de métriques
│   ├── span.rs                # Gestion des Spans
│   ├── event.rs               # Définition d'événements
│   ├── collector.rs           # Collecte de données
│   ├── exporter.rs            # Export de données
│   └── storage.rs             # Backend de stockage
│
├── tools/         # Système d'outils
│   ├── registry.rs             # Registre d'outils
│   ├── builtin_tools.rs        # Définition des outils intégrés
│   ├── builtin_handlers.rs     # Gestionnaires d'outils intégrés
│   ├── orchestration.rs        # Orchestration d'outils
│   ├── streaming.rs            # Sortie en streaming
│   ├── stats.rs                # Statistiques d'utilisation
│   ├── recorder.rs             # Enregistrement d'exécution
│   ├── agent_def_loader.rs     # Chargement de définitions d'agents
│   ├── agent_def_types.rs      # Types de définitions d'agents
│   ├── bash/                   # Outil Bash (analyseur/sandbox/sécurité/validation de chemins)
│   ├── hooks/                  # Hooks (registre/exécuteur)
│   ├── mcp/                    # Outils MCP (registre/OAuth/wrapper)
│   ├── permissions/            # Permissions (classificateur/règles/suivi)
│   └── tools/                  # Implémentations d'outils spécifiques
│       ├── agent.rs            # Outil agent
│       ├── bash.rs             # Exécution Bash
│       ├── context.rs          # Gestion de contexte
│       ├── cron.rs             # Planification Cron
│       ├── glob.rs             # Glob de fichiers
│       ├── grep.rs             # Recherche de contenu
│       ├── lsp.rs              # Outil LSP
│       ├── monitor.rs          # Outil de surveillance
│       ├── plan.rs             # Outil de plan
│       ├── repl.rs             # Outil REPL
│       ├── skill.rs            # Outil de compétence
│       ├── web_fetch.rs        # Extraction web
│       ├── web_search.rs       # Recherche web
│       ├── file_read.rs        # Lecture de fichier
│       ├── file_write.rs       # Écriture de fichier
│       ├── file_edit.rs        # Édition de fichier
│       ├── computer_use.rs     # Contrôle informatique
│       ├── messaging.rs        # Envoi de messages
│       ├── push_notification.rs # Notification push
│       ├── task_system.rs      # Système de tâches
│       ├── todo_write.rs       # Liste de tâches
│       └── batch_missing.rs    # Détection de lots manquants
│
├── trajectory/    # Système d'apprentissage
│   ├── memory.rs              # Gestion de la mémoire
│   ├── memory_provider.rs     # Interface de fournisseur de mémoire
│   ├── auto_memory.rs         # Extraction automatique de mémoire
│   ├── skill.rs               # Système de compétences
│   ├── skill_manager.rs       # Gestionnaire de compétences
│   ├── skill_evolution.rs     # Évolution des compétences
│   ├── skill_matcher.rs       # Correspondance des compétences
│   ├── skill_proposal.rs      # Proposition de compétences
│   ├── skills_hub_adapter.rs  # Adaptateur du hub de compétences
│   ├── skills_hub_client.rs   # Client du hub de compétences
│   ├── skill_decomposition/   # Décomposition des compétences (assistée LLM/multi-tours/validation flux de travail/analyse d'outils)
│   ├── rl.rs                  # Signaux de récompense RL
│   ├── rl_trainer.rs          # Entraîneur RL
│   ├── training_env.rs        # Environnement d'entraînement
│   ├── behavior_learner.rs    # Apprentissage comportemental
│   ├── behavior_tracker.rs    # Suivi comportemental
│   ├── pattern.rs             # Reconnaissance de patterns
│   ├── pattern_analyzer.rs    # Analyse de patterns
│   ├── user_profile.rs        # Profil utilisateur
│   ├── preference_learner.rs  # Apprentissage des préférences
│   ├── adaptation.rs          # Ajustement adaptatif
│   ├── dream_consolidation.rs # Intégration onirique
│   ├── parallel_execution.rs  # Service d'exécution parallèle
│   ├── style_extractor.rs     # Extraction de style
│   ├── style_applier.rs       # Application de style
│   ├── style_vectorizer.rs    # Vectorisation de style
│   ├── style_migrator.rs      # Migration de style
│   ├── suggestion_engine.rs   # Moteur de suggestions
│   ├── proactive_assistant.rs # Assistant proactif
│   ├── context_predictor.rs   # Prédiction de contexte
│   ├── task_prefetcher.rs     # Préchargement de tâches
│   ├── reminder_manager.rs    # Gestion des rappels
│   ├── nudge.rs               # Système de nudges
│   ├── insight.rs             # Génération d'insights
│   ├── compactor.rs           # Compression de données
│   ├── trajectory.rs          # Gestion de trajectoire
│   ├── trajectory_compressor.rs # Compression de trajectoire
│   ├── sub_agent.rs           # Sous-agent
│   ├── batch.rs               # Traitement par lots
│   ├── context.rs             # Gestion de contexte
│   ├── fts5.rs                # Recherche FTS5
│   ├── hooks.rs               # Hooks
│   ├── storage.rs             # Stockage
│   ├── scheduled_task.rs      # Tâche planifiée
│   └── memory_providers/      # Fournisseurs de mémoire (Honcho/Mem0/boucle fermée/services)
│
└── migration/     # Migrations de base de données
    └── m20240101_000001~000010  # 10 fichiers de migration
```

### Architecture Frontend

```
src/
├── stores/                    # Gestion d'état Zustand
│   ├── domain/               # État métier principal
│   │   ├── conversationStore.ts
│   │   ├── messageStore.ts
│   │   ├── streamStore.ts
│   │   ├── multiModelStore.ts
│   │   ├── preferenceStore.ts
│   │   └── compressStore.ts
│   ├── feature/               # État des modules fonctionnels (30+ stores)
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
│   ├── devtools/              # État des outils de développement
│   │   ├── tracerStore.ts
│   │   ├── evaluatorStore.ts
│   │   ├── rlStore.ts
│   │   ├── fineTuneStore.ts
│   │   └── recommendationStore.ts
│   └── shared/                # État partagé
│       ├── artifactStore.ts
│       ├── chatWorkspaceStore.ts
│       ├── tabStore.ts
│       └── uiStore.ts
│
├── components/                # Composants React (24 modules)
│   ├── chat/                # Interface de conversation (90+ composants)
│   ├── workflow/            # Éditeur de flux de travail (nœuds/panneaux/modèles/assistance IA)
│   ├── gateway/             # UI Passerelle API
│   ├── settings/            # Panneaux de paramètres (40+ composants)
│   ├── terminal/            # UI Terminal
│   ├── skill/               # Éditeur et rendu de compétences
│   ├── benchmark/           # Panneau de benchmarks
│   ├── decomposition/       # Décomposition de compétences et génération d'outils
│   ├── files/               # Page de gestion de fichiers
│   ├── fine-tune/           # Configuration d'ajustement LoRA
│   ├── link/                # Gestion des liens externes
│   ├── llm-wiki/            # Éditeur LLM Wiki
│   ├── proactive/           # Système de suggestions proactives
│   ├── recommendation/      # Panneau de recommandation d'outils
│   ├── wiki/                # Gestion Wiki
│   ├── devtools/            # Timeline Trace/Span
│   ├── style/               # Transfert de style de code
│   ├── layout/              # Composants de mise en page (barre de titre/barre latérale/palette de commandes)
│   ├── help/                # Panneau d'aide
│   ├── onboarding/          # Assistant d'intégration
│   ├── notification/        # Centre de notifications
│   ├── search/              # Recherche de sessions
│   ├── common/              # Composants communs
│   └── shared/              # Composants partagés
│
├── pages/                    # Composants de page (22 pages)
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
├── hooks/                    # React hooks (10)
├── lib/                      # Fonctions utilitaires (incluant Web Worker)
├── types/                    # Définitions de types TypeScript (22)
├── sdk/                      # SDK (incluant SDK Python)
└── i18n/                     # Traductions en 11 langues
```

### Support des plateformes

| Plateforme | Architecture |
|------------|-------------|
| macOS | Apple Silicon (arm64), Intel (x86_64) |
| Windows | x86_64, ARM64 |
| Linux | x86_64, ARM64 (AppImage/deb/rpm) |

## Démarrage rapide

### Télécharger les versions pré-construites

Visitez la page [Releases](https://github.com/polite0803/AxAgent/releases) pour télécharger le programme d'installation de votre plateforme.

### Compiler à partir du code source

#### Prérequis

- [Node.js](https://nodejs.org/) 20+
- [Rust](https://www.rust-lang.org/) 1.75+
- [npm](https://www.npmjs.com/) 10+
- Windows : [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) + cibles Rust MSVC

#### Étapes de compilation

```bash
# Cloner le dépôt
git clone https://github.com/polite0803/AxAgent.git
cd AxAgent

# Installer les dépendances
npm install

# Mode développement
npm run tauri dev

# Compiler uniquement le frontend
npm run build

# Compiler l'application desktop
npm run tauri build
```

Les artefacts de build se trouvent dans `src-tauri/target/release/`.

### Tests

```bash
# Tests unitaires
npm run test

# Tests E2E
npm run test:e2e

# Vérification des types
npm run typecheck

# Formatage du code
npm run format

# Vérification CI
npm run ci:check
```

---

## Structure du projet

```
AxAgent/
├── src/                         # Code source frontend (React + TypeScript)
│   ├── components/              # Composants React (24 modules)
│   │   ├── chat/               # Interface de conversation (90+ composants)
│   │   ├── workflow/           # Composants de l'éditeur de flux de travail
│   │   ├── gateway/            # Composants de la passerelle API
│   │   ├── settings/           # Panneaux de paramètres (40+ composants)
│   │   ├── terminal/           # Composants terminal
│   │   ├── skill/              # Éditeur et rendu de compétences
│   │   ├── benchmark/          # Benchmarks
│   │   ├── decomposition/      # Décomposition de compétences
│   │   ├── files/              # Gestion de fichiers
│   │   ├── fine-tune/          # Ajustement LoRA
│   │   ├── link/               # Liens externes
│   │   ├── llm-wiki/           # LLM Wiki
│   │   ├── proactive/          # Suggestions proactives
│   │   ├── recommendation/     # Recommandation d'outils
│   │   ├── wiki/               # Gestion Wiki
│   │   ├── devtools/           # Outils de développement
│   │   ├── style/              # Style de code
│   │   ├── layout/             # Composants de mise en page
│   │   ├── help/               # Panneau d'aide
│   │   ├── onboarding/         # Assistant d'intégration
│   │   ├── notification/       # Centre de notifications
│   │   ├── search/             # Recherche de sessions
│   │   ├── common/             # Composants communs
│   │   └── shared/             # Composants partagés
│   ├── pages/                   # Composants de page (22 pages)
│   ├── stores/                  # Gestion d'état Zustand
│   │   ├── domain/            # État métier principal (6 stores)
│   │   ├── feature/           # État des modules fonctionnels (30+ stores)
│   │   ├── devtools/          # État des outils de développement (5 stores)
│   │   └── shared/            # État partagé (4 stores)
│   ├── hooks/                   # React hooks (10)
│   ├── lib/                     # Fonctions utilitaires (incluant Web Worker)
│   ├── types/                   # Définitions de types TypeScript (22)
│   ├── sdk/                     # SDK (incluant SDK Python)
│   └── i18n/                    # Traductions en 11 langues
│
├── src-tauri/                    # Code source backend (Rust)
│   ├── crates/                  # Workspace Rust (10 crates)
│   │   ├── agent/             # Noyau de l'agent IA
│   │   ├── core/              # Base de données, chiffrement, RAG
│   │   ├── gateway/           # Serveur passerelle API
│   │   ├── plugins/           # Système de plugins
│   │   ├── providers/         # Adaptateurs de fournisseurs de modèles
│   │   ├── runtime/           # Services d'exécution
│   │   ├── tools/             # Système d'outils
│   │   ├── trajectory/        # Mémoire et apprentissage
│   │   ├── telemetry/         # Traçage et métriques
│   │   └── migration/         # Migrations de base de données
│   └── src/                    # Point d'entrée Tauri (70+ modules de commandes)
│
├── extension/                  # Extension de navigateur (Wiki Clipper)
├── e2e/                        # Tests E2E Playwright
├── scripts/                    # Scripts de build et d'outils
└── website/                    # Site web du projet (VitePress)
```

## Répertoire de données

```
~/.axagent/                      # Répertoire de configuration
├── axagent.db                   # Base de données SQLite
├── master.key                   # Clé maîtresse AES-256
├── vector_db/                   # Base de données vectorielle (sqlite-vec)
└── ssl/                         # Certificats SSL

~/Documents/axagent/            # Répertoire des fichiers utilisateur
├── images/                     # Images jointes
├── files/                      # Fichiers joints
└── backups/                    # Fichiers de sauvegarde
```

---

## FAQ

### macOS : « L'application est endommagée » ou « Impossible de vérifier le développeur »

Comme l'application n'est pas signée par Apple :

**1. Autoriser les applications de « N'importe où »**
```bash
sudo spctl --master-disable
```

Ensuite, allez dans **Réglages Système → Confidentialité et sécurité → Sécurité** et sélectionnez **N'importe où**.

**2. Supprimer l'attribut de quarantaine**
```bash
sudo xattr -dr com.apple.quarantine /Applications/AxAgent.app
```

**3. Étape supplémentaire pour macOS Ventura+**
Allez dans **Réglages Système → Confidentialité et sécurité**, puis cliquez sur **Ouvrir quand même**.

---

## Communauté

- [LinuxDO](https://linux.do)

## Licence

Ce projet est sous licence [AGPL-3.0](LICENSE).
