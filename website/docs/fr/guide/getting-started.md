# Démarrage rapide

## Installation

Téléchargez le dernier installateur depuis la [page de téléchargement](/fr/download) ou [GitHub Releases](https://github.com/polite0803/AxAgent/releases).

### macOS

| Puce | Fichier |
|------|---------|
| Apple Silicon (M1 / M2 / M3 / M4) | `AxAgent_x.x.x_aarch64.dmg` |
| Intel | `AxAgent_x.x.x_x64.dmg` |

1. Ouvrez le `.dmg` et faites glisser **AxAgent** dans le dossier **Applications**.
2. Lancez AxAgent. Si macOS bloque l'application, allez dans **Réglages Système → Confidentialité et sécurité** et cliquez sur **Ouvrir quand même**.

::: warning macOS : "L'app est endommagée" ou "Impossible de vérifier le développeur"
Si vous voyez l'un de ces messages, ouvrez Terminal et exécutez :

```bash
xattr -c /Applications/AxAgent.app
```

Relancez ensuite l'application. Cela supprime l'indicateur de quarantaine que macOS applique aux téléchargements non signés.
:::

### Windows

| Architecture | Fichier |
|-------------|---------|
| x64 (la plupart des PC) | `AxAgent_x.x.x_x64-setup.exe` |
| ARM64 | `AxAgent_x.x.x_arm64-setup.exe` |

Exécutez l'installateur et suivez l'assistant. Lancez AxAgent depuis le menu Démarrer ou le raccourci bureau.

### Linux

| Format | Architecture | Fichier |
|--------|-------------|---------|
| Debian / Ubuntu | x64 | `AxAgent_x.x.x_amd64.deb` |
| Debian / Ubuntu | ARM64 | `AxAgent_x.x.x_arm64.deb` |
| Fedora / openSUSE | x64 | `AxAgent_x.x.x_x86_64.rpm` |
| Fedora / openSUSE | ARM64 | `AxAgent_x.x.x_aarch64.rpm` |
| Toute distro | x64 | `AxAgent_x.x.x_amd64.AppImage` |
| Toute distro | ARM64 | `AxAgent_x.x.x_aarch64.AppImage` |

```bash
# Debian / Ubuntu
sudo dpkg -i AxAgent_x.x.x_amd64.deb

# Fedora / openSUSE
sudo rpm -i AxAgent_x.x.x_x86_64.rpm

# AppImage (toute distro)
chmod +x AxAgent_x.x.x_amd64.AppImage
./AxAgent_x.x.x_amd64.AppImage
```

---

## Configuration initiale

### 1. Ouvrir les paramètres

Lancez AxAgent et cliquez sur l'**icône d'engrenage** en bas de la barre latérale, ou appuyez sur <kbd>Cmd/Ctrl</kbd>+<kbd>,</kbd>.

### 2. Ajouter un fournisseur

Naviguez vers **Paramètres → Fournisseurs** et cliquez sur le bouton **+**.

1. Entrez un nom d'affichage (ex. *OpenAI*).
2. Sélectionnez le type de fournisseur (OpenAI, Anthropic, Google Gemini, etc.).
3. Collez votre clé API.
4. Confirmez l'**URL de base** — l'endpoint officiel est pré-rempli pour les types intégrés.

::: tip
Vous pouvez ajouter autant de fournisseurs que vous le souhaitez. Chaque fournisseur gère son propre ensemble de clés API et de modèles indépendamment.
:::

### 3. Récupérer les modèles

Cliquez sur **Récupérer les modèles** pour obtenir la liste des modèles disponibles depuis l'API du fournisseur. Vous pouvez également ajouter des IDs de modèles manuellement si nécessaire.

### 4. Définir un modèle par défaut

Allez dans **Paramètres → Modèle par défaut** et choisissez le fournisseur et le modèle que les nouvelles conversations devront utiliser par défaut.

---

## Votre première conversation

1. Cliquez sur **Nouveau chat** dans la barre latérale (ou appuyez sur <kbd>Cmd/Ctrl</kbd>+<kbd>N</kbd>).
2. Sélectionnez un modèle depuis le sélecteur de modèles en haut du chat.
3. Tapez un message et appuyez sur <kbd>Entrée</kbd>.
4. AxAgent diffuse la réponse en temps réel. Les modèles qui supportent les blocs de réflexion (ex. Claude, DeepSeek R1) affichent le processus de raisonnement dans une section repliable.

---

## Raccourcis

Voici les raccourcis clavier par défaut. Tous les raccourcis peuvent être personnalisés dans **Paramètres → Raccourcis**.

| Raccourci | Action |
|-----------|--------|
| <kbd>Cmd/Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>A</kbd> | Afficher / masquer la fenêtre actuelle |
| <kbd>Cmd/Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>Alt</kbd>+<kbd>A</kbd> | Afficher / masquer toutes les fenêtres |
| <kbd>Cmd/Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>W</kbd> | Fermer la fenêtre |
| <kbd>Cmd/Ctrl</kbd>+<kbd>N</kbd> | Nouvelle conversation |
| <kbd>Cmd/Ctrl</kbd>+<kbd>,</kbd> | Ouvrir les paramètres |
| <kbd>Cmd/Ctrl</kbd>+<kbd>K</kbd> | Palette de commandes |
| <kbd>Cmd/Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>G</kbd> | Basculer la passerelle API |

---

## Données et sauvegarde

### Répertoires de données

AxAgent stocke les données dans deux emplacements :

| Chemin | Contenu |
|--------|---------|
| `~/.axagent/` | État de l'application — base de données, clés de chiffrement, base vectorielle, certificats SSL |
| `~/Documents/axagent/` | Fichiers utilisateur — images, documents, sauvegardes |

### Sauvegarde automatique

Allez dans **Paramètres → Sauvegarde** pour configurer les sauvegardes automatiques vers un répertoire local ou un serveur WebDAV.

---

## Prochaines étapes

- [Configurer les fournisseurs](./providers) — ajouter et gérer les fournisseurs IA
- [Serveurs MCP](./mcp) — connecter des outils externes pour étendre les capacités IA
- [Passerelle API](./gateway) — exposer vos fournisseurs comme serveur API local
