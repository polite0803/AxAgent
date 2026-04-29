[简体中文](./README.md) | [繁體中文](./README-ZH-TW.md) | [English](./README-EN.md) | [日本語](./README-JA.md) | [한국어](./README-KO.md) | **Français** | [Deutsch](./README-DE.md) | [Español](./README-ES.md) | [Русский](./README-RU.md) | [हिन्दी](./README-HI.md) | [العربية](./README-AR.md)

[![AxAgent](https://github.com/polite0803/AxAgent/blob/main/src/assets/image/logo.png?raw=true)](https://github.com/polite0803/AxAgent)

<p align="center">
    <a href="https://www.producthunt.com/products/axagent?embed=true&amp;utm_source=badge-featured&amp;utm_medium=badge&amp;utm_campaign=badge-axagent" target="_blank" rel="noopener noreferrer"><img alt="AxAgent - Lightweight, high-perf cross-platform AI desktop client | Product Hunt" width="250" height="54" src="https://api.producthunt.com/widgets/embed-image/v1/featured.svg?post_id=1118403&amp;theme=light&amp;t=1775627359538"></a>
</p>

## Captures d'écran

| Rendu des graphiques de chat | Fournisseurs et modèles |
|:---:|:---:|
| ![](.github/images/s1-0412.png) | ![](.github/images/s2-0412.png) |

| Base de connaissances | Mémoire |
|:---:|:---:|
| ![](.github/images/s3-0412.png) | ![](.github/images/s4-0412.png) |

| Agent - Demande | Passerelle API en un clic |
|:---:|:---:|
| ![](.github/images/s5-0412.png) | ![](.github/images/s6-0412.png) |

| Sélection du modèle de chat | Navigation des chats |
|:---:|:---:|
| ![](.github/images/s7-0412.png) | ![](.github/images/s8-0412.png) |

| Agent - Approbation des permissions | Aperçu de la passerelle API |
|:---:|:---:|
| ![](.github/images/s9-0412.png) | ![](.github/images/s10-0412.png) |

## Fonctionnalités

### Chat et modèles

- **Support multi-fournisseurs** — Compatible avec OpenAI, Anthropic Claude, Google Gemini et toutes les API compatibles OpenAI
- **Gestion des modèles** — Récupération des listes de modèles distants, personnalisation des paramètres (température, tokens max, Top-P, etc.)
- **Rotation multi-clés** — Configurez plusieurs clés API par fournisseur avec rotation automatique pour distribuer la pression des limites de débit
- **Sortie en streaming** — Rendu en temps réel token par token avec blocs de réflexion repliables
- **Versions de messages** — Basculez entre plusieurs versions de réponse par message pour comparer les effets des modèles ou des paramètres
- **Ramification de conversation** — Créez de nouvelles branches à partir de n'importe quel nœud de message, avec comparaison côte à côte des branches
- **Gestion des conversations** — Épinglage, archivage, affichage groupé par temps et opérations en masse
- **Compression de conversation** — Compresse automatiquement les longues conversations en préservant les informations clés pour économiser l'espace de contexte
- **Réponse simultanée multi-modèles** — Posez la même question à plusieurs modèles simultanément avec comparaison côte à côte des réponses

### AI Agent

- **Support multi-fournisseurs** — Compatible avec OpenAI, Anthropic Claude, Google Gemini et toutes les API compatibles OpenAI, avec support d'Ollama pour les modèles locaux et des passerelles distantes comme OpenClaw/Hermes
- **Mode Agent** — Passez en mode Agent pour l'exécution autonome de tâches multi-étapes : lecture/écriture de fichiers, exécution de commandes, analyse de code, et plus
- **Trois niveaux de permissions** — Par défaut (écritures nécessitent approbation), Accepter les modifications (approbation automatique des modifications de fichiers), Accès complet (sans invite) — sûr et contrôlable
- **Sandbox de répertoire de travail** — Les opérations de l'Agent sont strictement confinées au répertoire de travail spécifié, empêchant tout accès non autorisé
- **Panneau d'approbation des outils** — Affichage en temps réel des demandes d'appel d'outils avec examen individuel, « toujours autoriser » en un clic, ou refuser
- **Suivi des coûts** — Statistiques d'utilisation des tokens et des coûts en temps réel par session

### Système multi-agents

- **Coordination sous-agent** — Créez plusieurs sous-agents formant une architecture de coordination maître-esclave
- **Exécution parallèle** — Traitez plusieurs agents en parallèle pour améliorer l'efficacité sur les tâches complexes
- **Débat contradictoire** — Plusieurs agents débattent de différents points de vue pour produire de meilleures solutions par la collision des idées
- **Moteur de flux de travail** — Orchestration puissante des flux de travail supportant les branchements conditionnels, les boucles et l'exécution parallèle
- **Rôles d'équipe** — Attribuez des rôles spécifiques aux différents agents (revue de code, tests, documentation, etc.) pour accomplir des tâches collaboratives

### Système de compétences

- **Marché des compétences** — Marché des compétences intégré pour parcourir et installer des compétences contribuées par la communauté
- **Création de compétences** — Créez des compétences à partir de propositions avec éditeur Markdown
- **Évolution des compétences** — L'IA analyse et améliore automatiquement les compétences existantes pour de meilleures performances
- **Correspondance des compétences** — Recommandation intelligente de compétences pertinentes, appliquées automatiquement aux scénarios de conversation appropriés
- **Enregistrement des compétences locales** — Support des outils personnalisés locaux enregistrés comme compétences
- **Hooks de plugin** — Support des hooks pre/post pour injecter une logique personnalisée avant/après l'exécution des compétences
- **Compétences atomiques** — Composants de compétences granulaires prenant en charge la construction de flux de travail complexes
- **Décomposition des compétences** — Décompose automatiquement les tâches complexes en compétences atomiques exécutables
- **Outils générés** — L'IA génère et enregistre automatiquement de nouveaux outils pour étendre les capacités de l'agent

### Système de flux de travail

- **Éditeur de flux de travail** — Concepteur visuel de flux de travail par glisser-déposer avec connexion et configuration de nœuds
- **Modèles de flux de travail** — Préréglages intégrés pour démarrer rapidement des tâches courantes
- **Gestion des versions** — Les modèles de flux de travail prennent en charge la gestion des versions avec possibilité de revenir à des versions historiques
- **Moteur de flux de travail** — Moteur d'exécution de flux de travail puissant prenant en charge l'exécution parallèle, conditionnelle et en boucle
- **Historique d'exécution** — Enregistrement détaillé de l'historique d'exécution des flux de travail avec suivi d'état et débogage
- **Assistance IA** — Assistance IA pour la conception de flux de travail, génération et optimisation automatiques

### Rendu de contenu

- **Rendu Markdown** — Prise en charge complète de la coloration syntaxique du code, des formules mathématiques LaTeX, des tableaux et des listes de tâches
- **Éditeur de code Monaco** — Monaco Editor intégré dans les blocs de code avec coloration syntaxique, copie et aperçu diff
- **Rendu de diagrammes** — Rendu intégré des diagrammes de flux Mermaid et des diagrammes d'architecture D2
- **Panneau Artifact** — Extraits de code, brouillons HTML, notes Markdown et rapports consultables dans un panneau dédié
- **Inspecteur de session** — Affichage en temps réel de la structure de session sous forme d'arborescence pour une navigation rapide vers n'importe quel message

### Recherche et connaissances

- **Recherche Web** — Intégré avec Tavily, Zhipu WebSearch, Bocha et plus, avec annotations de sources de citation
- **Base de connaissances locale (RAG)** — Prend en charge plusieurs bases de connaissances ; téléchargez des documents pour une analyse, un découpage et une indexation vectorielle automatiques, avec récupération sémantique des passages pertinents pendant les conversations
- **Graphe de connaissances** — Graphe de relations entité-connaissance pour visualiser les connexions entre les points de connaissance
- **Système de mémoire** — Mémoire multi-espace de noms avec entrée manuelle ou extraction automatique d'informations clés par IA
- **Recherche en texte intégral** — Moteur FTS5 pour la recherche rapide dans les conversations, fichiers et mémoires
- **Gestion du contexte** — Attachez de manière flexible des pièces jointes, des résultats de recherche, des passages de base de connaissances, des entrées de mémoire et des sorties d'outils

### Outils et extensions

- **Protocole MCP** — Implémentation complète du Model Context Protocol supportant les transports stdio et HTTP/WebSocket
- **Authentification OAuth** — Support du flux d'authentification OAuth pour les serveurs MCP
- **Outils intégrés** — Outils intégrés prêts à l'emploi pour les opérations de fichiers, l'exécution de code, la recherche et plus encore
- **Panneau d'exécution des outils** — Affichage visuel des requêtes d'appel d'outils et des résultats retournés
- **Client LSP** — Support intégré du protocole LSP pour la complétion de code intelligente et les diagnostics

### Passerelle API

- **Passerelle API locale** — Serveur API local intégré avec prise en charge native des interfaces OpenAI-compatible, Claude et Gemini
- **Liens externes** — Intégration en un clic avec des outils externes comme Claude CLI et OpenCode avec synchronisation automatique des clés API
- **Gestion des clés API** — Générez, révoquez et activez/désactivez les clés d'accès avec des notes descriptives
- **Analyses d'utilisation** — Analyse du volume de requêtes et de l'utilisation des tokens par clé, fournisseur et date
- **Outils de diagnostic** — Vérifications de santé de la passerelle, tests de connexion et débogage des requêtes
- **Support SSL/TLS** — Génération intégrée de certificats auto-signés, avec prise en charge des certificats personnalisés
- **Journaux des requêtes** — Enregistrement complet de toutes les requêtes et réponses API passant par la passerelle
- **Modèles de configuration** — Modèles d'intégration pré-construits pour les outils CLI populaires tels que Claude, Codex, OpenCode et Gemini
- **Communication en temps réel** — Push d'événements WebSocket en temps réel, compatible avec l'API OpenAI Realtime

### Données et sécurité

- **Chiffrement AES-256** — Les clés API et les données sensibles sont chiffrées localement avec AES-256-GCM
- **Répertoires de données isolés** — État de l'application dans `~/.axagent/` ; fichiers utilisateur dans `~/Documents/axagent/`
- **Sauvegarde automatique** — Sauvegardes automatiques planifiées vers des répertoires locaux ou un stockage WebDAV
- **Restauration de sauvegarde** — Restauration en un clic à partir des sauvegardes historiques
- **Export de conversation** — Exportez les conversations en captures PNG, Markdown, texte brut ou JSON
- **Gestion de l'espace de stockage** — Affichage visuel de l'utilisation du disque avec nettoyage des fichiers inutiles

### Expérience bureau

- **Changement de thème** — Thèmes sombre/clair qui suivent les préférences du système ou peuvent être définis manuellement
- **Langue d'interface** — Prise en charge complète du chinois simplifié, du chinois traditionnel, de l'anglais, du japonais, du coréen, du français, de l'allemand, de l'espagnol, du russe, de l'hindi et de l'arabe
- **Barre d'état système** — Réduction dans la barre d'état système à la fermeture de la fenêtre sans interrompre les services en arrière-plan
- **Toujours au premier plan** — Épinglez la fenêtre principale pour qu'elle reste au-dessus de toutes les autres fenêtres
- **Raccourcis globaux** — Raccourcis clavier globaux personnalisables pour appeler la fenêtre principale à tout moment
- **Démarrage automatique** — Lancement optionnel au démarrage du système
- **Support proxy** — Configuration de proxy HTTP et SOCKS5
- **Mise à jour automatique** — Vérifie automatiquement les nouvelles versions au démarrage et invite à la mise à jour
- **Palette de commandes** — `Cmd/Ctrl+K` pour accéder rapidement à toutes les commandes et paramètres

## Plateformes prises en charge

| Plateforme | Architecture |
|------------|-------------|
| macOS | Apple Silicon (arm64), Intel (x86_64) |
| Windows 10/11 | x86_64, arm64 |
| Linux | x86_64 (AppImage/deb/rpm), arm64 (AppImage/deb/rpm) |

## Architecture Technique

### Pile Technologique

| Couche | Technologie |
|--------|-------------|
| **Framework** | Tauri 2 + React 19 + TypeScript |
| **UI** | Ant Design 6 + TailwindCSS 4 |
| **State** | Zustand 5 |
| **i18n** | i18next + react-i18next |
| **Backend** | Rust + SeaORM + SQLite |
| **Vector DB** | sqlite-vec |
| **Code Editor** | Monaco Editor |
| **Diagrammes** | Mermaid + D2 + ECharts |
| **Terminal** | xterm.js |
| **Build** | Vite + npm |

### Architecture Backend Rust

Le backend est organisé comme un workspace Rust avec des crates spécialisées:

```
src-tauri/crates/
├── agent/         # Cœur de l'Agent IA
│   ├── react_engine.rs       # Moteur de raisonnement ReAct
│   ├── tool_registry.rs      # Enregistrement dynamique des outils
│   ├── coordinator.rs        # Coordination des agents
│   ├── hierarchical_planner.rs # Décomposition des tâches
│   ├── self_verifier.rs      # Vérification des sorties
│   ├── error_recovery_engine.rs # Gestion des erreurs
│   ├── vision_pipeline.rs    # Perception visuelle
│   └── fine_tune/            # Ajustement LoRA
│
├── core/          # Utilitaires principaux
│   ├── db.rs               # Base de données SeaORM
│   ├── vector_store.rs      # Intégration sqlite-vec
│   ├── rag.rs              # Couche d'abstraction RAG
│   ├── hybrid_search.rs    # Recherche hybride vecteur + FTS5
│   ├── crypto.rs           # Chiffrement AES-256
│   └── mcp_client.rs       # Client protocole MCP
│
├── gateway/       # Passerelle API
│   ├── server.rs           # Serveur HTTP
│   ├── handlers.rs         # Gestionnaires API
│   ├── auth.rs             # Authentification
│   └── realtime.rs         # Support WebSocket
│
├── providers/     # Adaptateurs de modèles
│   ├── openai.rs          # API OpenAI
│   ├── anthropic.rs       # API Claude
│   ├── gemini.rs          # API Gemini
│   └── ollama.rs          # Ollama local
│
├── runtime/       # Services runtime
│   ├── session.rs         # Gestion des sessions
│   ├── workflow_engine.rs  # Orchestration DAG
│   ├── mcp.rs             # Serveur MCP
│   ├── cron/              # Planification des tâches
│   ├── terminal/          # Terminaux backend
│   ├── shell_hooks.rs     # Intégration Shell
│   └── message_gateway/   # Intégrations plateforme
│
└── trajectory/   # Système d'apprentissage
    ├── memory.rs          # Gestion de la mémoire
    ├── skill.rs           # Système de compétences
    ├── rl.rs              # Signaux de récompense RL
    ├── behavior_learner.rs # Apprentissage des patterns
    └── user_profile.rs    # Profilage utilisateur
```

### Architecture Frontend

```
src/
├── stores/                    # Gestion d'état Zustand
│   ├── domain/               # État métier principal
│   │   ├── conversationStore.ts
│   │   ├── messageStore.ts
│   │   └── streamStore.ts
│   ├── feature/              # État des modules fonctionnels
│   │   ├── agentStore.ts
│   │   ├── gatewayStore.ts
│   │   ├── workflowEditorStore.ts
│   │   └── knowledgeStore.ts
│   └── shared/               # État partagé
│
├── components/
│   ├── chat/                # Interface de chat (60+ composants)
│   ├── workflow/            # Éditeur de workflow
│   ├── gateway/             # UI Passerelle API
│   ├── settings/            # Panneaux de paramètres
│   └── terminal/            # UI Terminal
│
└── pages/                   # Composants de page
```

## Démarrage rapide

Rendez-vous sur la page [Releases](https://github.com/polite0803/AxAgent/releases) et téléchargez le programme d'installation pour votre plateforme.

## Compiler à partir du code source

### Prérequis

- [Node.js](https://nodejs.org/) 20+
- [Rust](https://www.rust-lang.org/) 1.75+
- [npm](https://www.npmjs.com/) 10+
- Windows nécessite [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) et [Rust MSVC targets](https://doc.rust-lang.org/cargo/reference/config.html#cfgtarget)

### Étapes de compilation

```bash
# Cloner le dépôt
git clone https://github.com/polite0803/AxAgent.git
cd AxAgent

# Installer les dépendances
npm install

# Exécuter en mode développement
npm run tauri dev

# Compiler uniquement le frontend
npm run build

# Compiler l'application desktop
npm run tauri build
```

Les artefacts de build se trouvent dans le répertoire `src-tauri/target/release/`.

### Tests

```bash
# Exécuter les tests unitaires
npm test

# Exécuter les tests end-to-end
npm run test:e2e

# Vérification des types
npm run typecheck
```

## FAQ

### macOS : « L'application est endommagée » ou « Impossible de vérifier le développeur »

Comme l'application n'est pas signée par Apple, macOS peut afficher l'une des invites suivantes :

- « AxAgent » est endommagé et ne peut pas être ouvert
- « AxAgent » ne peut pas être ouvert car Apple ne peut pas vérifier l'absence de logiciels malveillants

**Étapes pour résoudre le problème :**

**1. Autoriser les applications de « N'importe où »**

```bash
sudo spctl --master-disable
```

Ensuite, allez dans **Réglages Système → Confidentialité et sécurité → Sécurité** et sélectionnez **N'importe où**.

**2. Supprimer l'attribut de quarantaine**

```bash
sudo xattr -dr com.apple.quarantine /Applications/AxAgent.app
```

> Astuce : Vous pouvez faire glisser l'icône de l'application dans le terminal après avoir tapé `sudo xattr -dr com.apple.quarantine `.

**3. Étape supplémentaire pour macOS Ventura et versions ultérieures**

Après avoir effectué les étapes ci-dessus, le premier lancement peut encore être bloqué. Allez dans **Réglages Système → Confidentialité et sécurité**, puis cliquez sur **Ouvrir quand même** dans la section Sécurité. Cette opération n'est nécessaire qu'une seule fois.

## Communauté
- [LinuxDO](https://linux.do)

## Licence

Ce projet est sous licence [AGPL-3.0](LICENSE).