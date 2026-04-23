# त्वरित प्रारंभ

## इंस्टॉलेशन

[डाउनलोड पेज](/hi/download) या [GitHub Releases](https://github.com/polite0803/AxAgent/releases) से नवीनतम इंस्टॉलर डाउनलोड करें।

### macOS

| चिप | फ़ाइल |
|-----|-------|
| Apple Silicon (M1 / M2 / M3 / M4) | `AxAgent_x.x.x_aarch64.dmg` |
| Intel | `AxAgent_x.x.x_x64.dmg` |

1. `.dmg` खोलें और **AxAgent** को **Applications** फोल्डर में ड्रैग करें।
2. AxAgent लॉन्च करें। अगर macOS ऐप को ब्लॉक करे, **System Settings → Privacy & Security** में जाएं और **Open Anyway** क्लिक करें।

::: warning macOS: "App Is Damaged" या "Cannot Verify Developer"
अगर ये संदेश दिखाई दें, तो Terminal खोलें और चलाएं:

```bash
xattr -c /Applications/AxAgent.app
```

फिर ऐप दोबारा लॉन्च करें।
:::

### Windows

| आर्किटेक्चर | फ़ाइल |
|------------|-------|
| x64 (अधिकांश PC) | `AxAgent_x.x.x_x64-setup.exe` |
| ARM64 | `AxAgent_x.x.x_arm64-setup.exe` |

इंस्टॉलर चलाएं और विज़ार्ड का पालन करें।

### Linux

```bash
# Debian / Ubuntu
sudo dpkg -i AxAgent_x.x.x_amd64.deb

# Fedora / openSUSE
sudo rpm -i AxAgent_x.x.x_x86_64.rpm

# AppImage
chmod +x AxAgent_x.x.x_amd64.AppImage
./AxAgent_x.x.x_amd64.AppImage
```

---

## प्रारंभिक सेटअप

### 1. Settings खोलें

AxAgent लॉन्च करें और साइडबार के नीचे **gear आइकन** क्लिक करें, या <kbd>Cmd/Ctrl</kbd>+<kbd>,</kbd> दबाएं।

### 2. प्रदाता जोड़ें

**Settings → Providers** में जाएं और **+** बटन क्लिक करें।

1. एक डिस्प्ले नेम दर्ज करें (जैसे *OpenAI*)।
2. प्रदाता प्रकार चुनें (OpenAI, Anthropic, Google Gemini, आदि)।
3. अपनी API कुंजी पेस्ट करें।
4. **Base URL** कन्फर्म करें — बिल्ट-इन प्रकारों के लिए आधिकारिक एंडपॉइंट पहले से भरा है।

::: tip
आप जितने चाहें उतने प्रदाता जोड़ सकते हैं।
:::

### 3. मॉडल फ़ेच करें

**Fetch Models** क्लिक करें प्रदाता की API से उपलब्ध मॉडलों की सूची प्राप्त करने के लिए।

### 4. डिफ़ॉल्ट मॉडल सेट करें

**Settings → Default Model** में जाएं और प्रदाता और मॉडल चुनें जो नई वार्तालाप डिफ़ॉल्ट रूप से उपयोग करें।

---

## आपकी पहली वार्तालाप

1. साइडबार में **New Chat** क्लिक करें (या <kbd>Cmd/Ctrl</kbd>+<kbd>N</kbd> दबाएं)।
2. चैट के शीर्ष पर मॉडल सेलेक्टर से एक मॉडल चुनें।
3. एक संदेश टाइप करें और <kbd>Enter</kbd> दबाएं।
4. AxAgent रियल-टाइम में रेस्पॉन्स स्ट्रीम करता है।

---

## शॉर्टकट

| शॉर्टकट | क्रिया |
|---------|--------|
| <kbd>Cmd/Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>A</kbd> | वर्तमान विंडो दिखाएं / छुपाएं |
| <kbd>Cmd/Ctrl</kbd>+<kbd>N</kbd> | नई वार्तालाप |
| <kbd>Cmd/Ctrl</kbd>+<kbd>,</kbd> | Settings खोलें |
| <kbd>Cmd/Ctrl</kbd>+<kbd>K</kbd> | Command palette |
| <kbd>Cmd/Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>G</kbd> | API गेटवे टॉगल करें |

---

## डेटा और बैकअप

### डेटा डायरेक्टरी

| पथ | सामग्री |
|----|---------|
| `~/.axagent/` | एप्लिकेशन स्टेट — डेटाबेस, एन्क्रिप्शन कीज़, वेक्टर DB, SSL सर्टिफिकेट |
| `~/Documents/axagent/` | यूज़र फ़ाइलें — इमेज, दस्तावेज़, बैकअप |

---

## अगले कदम

- [प्रदाता कॉन्फ़िगर करें](./providers) — AI प्रदाता जोड़ें और प्रबंधित करें
- [MCP सर्वर](./mcp) — AI क्षमताओं को बढ़ाने के लिए बाहरी टूल्स कनेक्ट करें
- [API गेटवे](./gateway) — अपने प्रदाताओं को लोकल API सर्वर के रूप में एक्सपोज़ करें
