[English](./README-EN.md) | [简体中文](./README.md) | [繁體中文](./README-ZH-TW.md) | [日本語](./README-JA.md) | [한국어](./README-KO.md) | **Français** | [Deutsch](./README-DE.md) | [Español](./README-ES.md) | [Русский](./README-RU.md) | [हिन्दी](./README-HI.md) | [العربية](./README-AR.md)

[![AxAgent](https://github.com/polite0803/AxAgent/blob/main/src/assets/image/logo.png?raw=true)](https://github.com/polite0803/AxAgent)

<p align="center">
  <a href="https://www.producthunt.com/products/axagent?embed=true&amp;amp;utm_source=badge-featured&amp;amp;&amp;#10;&amp;amp;amp;;utm_medium=badge&amp;amp;#10&amp;amp;;utm_campaign=badge-axagent" target="_blank" rel="noopener noreferrer"><img alt="AxAgent - Lightweight, high-perf cross-platform AI desktop client | Product Hunt" width="250" height="54" src="https://api.producthunt.com/widgets/embed-image/v1/featured.svg?post_id=1118403&amp;theme=light&amp;t=1775627359538"></a>
</p>

<p align="center">
  <strong>Client desktop/mobile IA multiplateforme | Collaboration multi-agents | Local d'abord</strong>
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

## Qu'est-ce qu'AxAgent ?

**AxAgent v2.0** est une application desktop/mobile IA multiplateforme complète, intégrant des capacités d'agents IA avancées et des outils de développement riches. Elle prend en charge plusieurs fournisseurs de modèles, l'exécution autonome de pipelines, l'orchestration visuelle de flux de travail, la gestion locale des connaissances et une passerelle API intégrée, couvrant les plateformes **Windows / macOS / Linux / Android / iOS**.

---

## Aperçu des captures d'écran

| Conversation et sélection de modèle |  Tableau de bord multi-agents   |
| :---------------------------------: | :-----------------------------: |
|   ![](.github/images/s1-0412.png)   | ![](.github/images/s5-0412.png) |

|    Base de connaissances RAG    |       Mémoire et contexte       |
| :-----------------------------: | :-----------------------------: |
| ![](.github/images/s3-0412.png) | ![](.github/images/s4-0412.png) |

|   Éditeur de flux de travail    |          Passerelle API          |
| :-----------------------------: | :------------------------------: |
| ![](.github/images/s9-0412.png) | ![](.github/images/s10-0412.png) |

---

## Fonctionnalités principales

### 🤖 Prise en charge des modèles IA

- **Support multi-fournisseurs** — Intégration native d'OpenAI, Anthropic Claude, Google Gemini, Ollama, OpenClaw, Hermes et de toutes les API compatibles OpenAI
- **Rotation multi-clés** — Configurez plusieurs clés API par fournisseur avec rotation automatique pour distribuer la pression des limites de débit
- **Support des modèles locaux** — Prise en charge complète des modèles locaux Ollama, incluant la gestion des fichiers GGUF/GGML
- **Moteur d'inférence Candle** — Inférence locale Candle intégrée avec interfaces rerank/judge et téléchargement GGUF à la demande
- **Gestion des modèles** — Récupération des listes de modèles distants, personnalisation des paramètres (température, tokens max, top-p, etc.)
- **Sortie en streaming** — Rendu en temps réel token par token avec blocs de réflexion repliables (pensée étendue Claude)
- **Comparaison multi-modèles** — Posez la même question à plusieurs modèles simultanément avec comparaison côte à côte
- **Appel de fonctions** — Appels de fonctions structurés sur tous les fournisseurs pris en charge
- **API Responses OpenAI** — Prise en charge du transport au format OpenAI Responses
- **API Realtime** — Push d'événements WebSocket compatible avec l'API Realtime OpenAI
- **Génération d'images IA** — DALL-E 3 et Flux (Replicate), préréglages de taille multiples (1:1/16:9/9:16/4:3), prompts négatifs
- **Routage intelligent de modèles** — Routage automatique par type de tâche (revue de code/résumé/traduction), règles de routage personnalisées
- **Appel vocal** — Conversation vocale en temps réel via l'API Realtime OpenAI, basculement d'état connexion/parole/écoute

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
- **Panneau du pool d'agents** — Visualisation en temps réel de l'état des sous-agents/Workers/étapes de flux de travail
- **Panneau de réflexion d'agent** — Notation de qualité post-tâche, analyse d'efficacité, patterns d'erreurs, suggestions d'amélioration
- **Sélecteur d'experts** — Import/export/personnalisation des rôles d'experts, filtrage par catégorie, préréglages intégrés
- **Arborescence hiérarchique d'agents** — Visualisation de la hiérarchie et de la topologie de collaboration des agents
- **Classificateur d'intention** — Identification automatique du type d'intention des entrées utilisateur
- **Gestion d'état de croyance** — Maintien de l'état de compréhension du contexte de l'agent
- **Évaluateur d'objectifs** — Évaluation de l'accomplissement et de la qualité des objectifs de tâche
- **Gestion de fenêtre de contexte** — Gestion intelligente de la fenêtre de contexte, optimisation de l'utilisation des tokens
- **Mémoire de projet** — Persistance des connaissances au niveau projet entre les sessions
- **Gestion de base de connaissances** — Opérations CRUD de base de connaissances
- **Système de notes** — Stockage et récupération de notes structurées au sein des agents

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
- **Panneau de collaboration** — Gestion de sessions de collaboration en temps réel, partage par code d'invitation, rôles des participants (Owner/Editor/Viewer)
- **Partage de session** — Lien de partage en un clic, configuration des permissions d'accès terminal/fichier/modèle

### ⭐ Système de compétences

- **Marché des compétences** — Marché intégré pour parcourir et installer des compétences contribuées par la communauté
- **Création de compétences** — Création automatique de compétences à partir de propositions, avec éditeur Markdown
- **Évolution des compétences** — Analyse et amélioration automatiques pilotées par l'IA des compétences existantes basées sur les retours d'exécution
- **Panneau d'évolution des compétences** — Visualisation des générations d'évolution, meilleure/moyenne fitness, état de convergence
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
- **Self-RAG** — Génération augmentée par auto-récupération, détermination intelligente de la nécessité de récupération et de la pertinence des résultats
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
- **Système de plugins** — Architecture de plugins à trois niveaux compatible OpenClaw (intégré/lié/externe), avec installation de paquets npm, enregistrement d'outils, hooks et gestion du cycle de vie
- **Marché de plugins** — Interface de marché intégrée avec recherche npm, installation et dialogues de confirmation
- **Outils intégrés** — Opérations de fichiers complètes (lecture/écriture/édition), exécution de code, recherche (Grep/Glob), Bash, recherche web, extraction web, gestion de plans, planification Cron, REPL, LSP, gestion de contexte, contrôle informatique, envoi de messages, liste de tâches, etc.
- **Système de permissions d'outils** — Classification des permissions d'outils, gestion des règles et suivi de l'utilisation
- **Sécurité Bash** — Analyse de commandes, validation de chemins et contrôle de sécurité sandbox
- **Client LSP** — Protocole Language Server intégré, complétion de code et diagnostics
- **Index AST** — Analyse et indexation AST des fichiers de code
- **Backend terminal** — Support des connexions terminal locales, Docker et SSH
- **Automatisation de navigateur** — Contrôle de navigateur via CDP (navigation, captures d'écran, clics, remplissage, extraction de texte, etc.)
- **Automatisation UI** — Identification et contrôle d'éléments UI multiplateforme
- **Outils Git** — Opérations Git avec détection de branches et sensibilité aux conflits
- **Panneau de commit Git** — Statistiques diff Git visuelles, messages de commit générés par l'IA, staging et commit en un clic
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
- **Interpréteur de graphiques** — Analyse et visualisation de données graphiques par l'IA (barres/lignes/camembert/dispersion/aire), insights automatiques
- **Visionneuse Diff** — Comparaison de versions de conversation, Accepter/Rejeter par fichier, détection automatique de langue
- **Barre de classification de contexte** — Affichage segmenté de l'utilisation des tokens de contexte par catégorie
- **Graphe de contexte** — Visualisation ReactFlow des relations de contexte
- **Suggestion de commandes** — Suggestion automatique de commandes pendant la saisie
- **Gestionnaire de citations** — Suivi/classification des sources de citation avec notation de crédibilité
- **Badge de crédibilité** — Visualisation de crédibilité à cinq étoiles

### 🛡️ Données et sécurité

- **Chiffrement AES-256** — Clés API et données sensibles chiffrées avec AES-256-GCM
- **Stockage isolé** — État de l'application dans `~/.axagent/`, fichiers utilisateur dans `~/Documents/axagent/`
- **Sauvegarde automatique** — Sauvegardes planifiées vers un répertoire local ou un stockage WebDAV
- **Espace de travail cloud** — Synchronisation de stockage cloud S3 et WebDAV, détection/résolution de conflits, synchronisation bidirectionnelle
- **Restauration de sauvegarde** — Restauration en un clic depuis les sauvegardes historiques
- **Options d'export** — Captures PNG, Markdown, texte brut, JSON
- **Gestion du stockage** — Affichage visuel de l'utilisation du disque et outils de nettoyage
- **Autorisation de fichiers** — Gestion des autorisations et révocation de l'accès aux fichiers
- **Audit des opérations** — Journal d'audit des opérations critiques

### 🖥️ Expérience bureau

- **Mise en page responsive** — Auto-adaptation trois niveaux desktop/tablette/mobile (points de rupture 600px/900px), basculement en temps réel au redimensionnement
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
- **Indicateur d'état onirique** — Affichage en temps réel de l'état et des résultats de l'intégration onirique
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

### 🛡️ Protection contre l'injection de prompt (Prompt-Guard)

- **Protection à quatre niveaux** — L1 détection de motifs (blocage haut risque + marquage risque moyen) → L2 échappement de délimiteurs → L3 enveloppe XML → L4 étiquettes de confiance
- **Orchestrateur de pipeline** — Pipeline de détection multi-niveaux avec seuils de risque personnalisables
- **Détection de contrebande de tokens** — Détection spécialisée contre l'obfuscation d'encodage et les attaques de contrebande de tokens
- **Mode Strict** — Tests en mode strict + nommage des raisons à risque moyen + documentation des modes personnalisés
- **Intégration complète du pipeline** — Intégré dans les flux session / prompt / git / RAG

### 📱 Support mobile

- **Android natif** — Builds APK/AAB, support arm64-v8a / armeabi-v7a / x86_64
- **iOS natif** — Builds IPA, support arm64
- **Mise en page adaptative** — Auto-adaptation trois niveaux desktop/tablette/téléphone (points de rupture CSS 600px/900px, basculement en temps réel au redimensionnement de fenêtre)
- **Navigation mobile** — Navigation Drawer coulissante + barre de navigation inférieure + FAB flash
- **Adaptation zone sûre** — Adaptation CSS env() barre d'état/barre de navigation Android
- **Optimisation CSP** — Liste blanche de protocole CSP Android WebView

---

## Architecture technique

### Pile technologique

| Couche               | Technologie                                            |
| -------------------- | ------------------------------------------------------ |
| **Framework**        | Tauri 2 + React 19 + TypeScript 6                      |
| **UI**               | Ant Design 6 + TailwindCSS 4                           |
| **Gestion d'état**   | Zustand 5                                              |
| **Routage**          | React Router 7                                         |
| **i18n**             | i18next + react-i18next                                |
| **Backend**          | Rust + SeaORM 2 + SQLite                               |
| **Base vectorielle** | sqlite-vec                                             |
| **Éditeur de code**  | Monaco Editor                                          |
| **Diagrammes**       | Mermaid + D2 + ECharts (CDN)                           |
| **Terminal**         | xterm.js 6                                             |
| **Flux de travail**  | ReactFlow 11                                           |
| **Infographie**      | @antv/infographic                                      |
| **Icônes**           | Iconify + Lucide                                       |
| **Glisser-déposer**  | @dnd-kit                                               |
| **Build**            | Vite 8 + npm                                           |
| **Tests**            | Vitest + Playwright + cargo-nextest                    |
| **Formatage**        | dprint (TS/JSON) + rustfmt                             |
| **Lint**             | TS: eslint + oxlint / Rust: clippy + cargo-deny        |
| **Mobile**           | Builds natifs Tauri Android + iOS                      |
| **Desktop**          | Windows (MSI) · macOS (DMG) · Linux (AppImage/deb/rpm) |

### Support des plateformes

| Plateforme | Architecture                               |
| ---------- | ------------------------------------------ |
| Windows    | x86_64, ARM64                              |
| macOS      | Apple Silicon (arm64), Intel (x86_64)      |
| Linux      | x86_64, ARM64                              |
| Android    | arm64-v8a, armeabi-v7a, x86_64 (émulateur) |
| iOS        | arm64                                      |

### Architecture Backend Rust

Le backend est organisé comme un workspace Rust avec **18 crates** spécialisées :

```
src-tauri/crates/
├── agent/            # Noyau de l'agent IA (moteur ReAct, coordination, planification, recherche approfondie, vérification des faits, etc.)
├── core/             # Utilitaires principaux (base de données, RAG, chiffrement, MCP, automatisation navigateur, index AST, etc.)
├── providers/        # Adaptateurs de fournisseurs de modèles (OpenAI, Anthropic, Gemini, Ollama, OpenClaw, etc.)
├── runtime-core/     # Couche d'abstraction d'exécution (types communs, définitions de traits, configuration)
├── runtime/          # Services d'exécution (gestion des sessions, MCP, terminal, limitation de débit, Webhooks, permissions, etc.)
├── rt-workflow/      # Moteur de flux de travail (orchestration DAG, exécuteurs de nœuds, planificateur)
├── rt-messaging/     # Passerelle de messagerie (intégration DingTalk/Feishu/QQ/Slack/WeChat/WhatsApp/Telegram/Discord)
├── rt-webhook/       # Serveur et distribution Webhook
├── rt-dashboard/     # Système de plugins de tableau de bord
├── rt-theme/         # Moteur de thèmes
├── gateway/          # Passerelle API (serveur HTTP, authentification, routes, interface compatible OpenAI)
├── tools/            # Système d'outils (registre, orchestration, sortie en streaming, 40+ outils intégrés)
├── trajectory/       # Système d'apprentissage (mémoire, compétences, RL, profil utilisateur, intégration onirique)
├── telemetry/        # Télémétrie et traçage distribué
├── plugins/          # Système de plugins (compatible OpenClaw, installation de paquets npm)
├── prompt-guard/     # Protection contre l'injection de prompt (détection et défense multi-niveaux L1-L4)
├── migration/        # Migrations de base de données
├── npm/              # Analyse de paquets npm et registre
└── code_engine/      # Moteur d'inférence local Candle (déprécié, fonctionnalités intégrées dans core)
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
npm run test          # Vitest watch
npm run test:run      # Vitest exécution unique

# Tests E2E
npm run test:e2e      # Playwright
npm run test:e2e:ui   # Playwright mode UI

# Tests backend Rust
cd src-tauri && cargo nextest run   # cargo-nextest (2-3x plus rapide)
cd src-tauri && cargo test          # Tests standard

# Vérification des types
npm run typecheck     # TypeScript
cd src-tauri && cargo clippy -- -D warnings  # Rust

# Formatage du code
npm run format        # dprint
cd src-tauri && cargo fmt

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
│   ├── pages/                   # Composants de page (18 pages)
│   ├── stores/                  # Gestion d'état Zustand (62 stores)
│   │   ├── domain/            # État métier principal (9 stores)
│   │   ├── feature/           # État des modules fonctionnels (44 stores)
│   │   ├── devtools/          # État des outils de développement (5 stores)
│   │   └── shared/            # État partagé (4 stores)
│   ├── hooks/                   # React hooks
│   ├── lib/                     # Fonctions utilitaires (incluant Web Worker)
│   ├── types/                   # Définitions de types TypeScript
│   ├── sdk/                     # SDK (incluant SDK Python)
│   └── i18n/                    # Traductions en 11 langues
│
├── src-tauri/                    # Code source backend (Rust)
│   ├── crates/                  # Workspace Rust (18 crates)
│   │   ├── agent/             # Noyau de l'agent IA
│   │   ├── core/              # Base de données, chiffrement, RAG, MCP
│   │   ├── providers/         # Adaptateurs de fournisseurs de modèles
│   │   ├── runtime-core/      # Couche d'abstraction d'exécution
│   │   ├── runtime/           # Services d'exécution
│   │   ├── rt-workflow/       # Moteur de flux de travail
│   │   ├── rt-messaging/      # Passerelle de messagerie
│   │   ├── rt-webhook/        # Serveur Webhook
│   │   ├── rt-dashboard/      # Plugins de tableau de bord
│   │   ├── rt-theme/          # Moteur de thèmes
│   │   ├── gateway/           # Serveur passerelle API
│   │   ├── tools/             # Système d'outils
│   │   ├── trajectory/        # Mémoire et apprentissage
│   │   ├── telemetry/         # Traçage et métriques
│   │   ├── plugins/           # Système de plugins
│   │   ├── prompt-guard/      # Protection contre l'injection de prompt
│   │   ├── migration/         # Migrations de base de données
│   │   └── npm/               # Analyse de paquets npm
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
