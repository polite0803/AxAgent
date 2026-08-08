// SPDX-License-Identifier: AGPL-3.0-only

/** Injection type */
export type InjectionType =
  | "prompt_injection"
  | "sql_injection"
  | "command_injection"
  | "path_traversal"
  | "xss"
  | "exfiltration"
  | "role_play_attack"
  | "jailbreak";

/** Injection detection result */
export interface InjectionDetection {
  type: InjectionType;
  severity: "low" | "medium" | "high" | "critical";
  matchedPattern: string;
  position: number;
  context: string;
  confidence: number;
  recommendation: string;
}

/** Sanitization result */
export interface SanitizationResult {
  isClean: boolean;
  sanitizedText: string;
  detections: InjectionDetection[];
  riskScore: number;
}

/** Detection rules */
interface DetectionRule {
  type: InjectionType;
  patterns: RegExp[];
  severity: "low" | "medium" | "high" | "critical";
  recommendation: string;
}

const DETECTION_RULES: DetectionRule[] = [
  {
    type: "prompt_injection",
    severity: "high",
    recommendation: "Potential prompt injection attack detected. Review input content carefully.",
    patterns: [
      /ignore\s+(all\s+)?(previous|prior)\s+(instructions?|prompts?|rules?)/i,
      /disregard\s+(all\s+)?(previous|prior)\s+(instructions?|prompts?|rules?)/i,
      /forget\s+(all\s+)?(previous|prior)\s+(instructions?|prompts?|rules?)/i,
      /you\s+are\s+now\s+(a|an)\s+/i,
      /system\s*:\s*/i,
      /user\s*:\s*/i,
      /assistant\s*:\s*/i,
      /```system[\s\S]*?```/i,
      /prompt\s*injection|jailbreak|bypass/i,
      /ignore\s+your\s+(instructions?|rules?|training)/i,
    ],
  },
  {
    type: "sql_injection",
    severity: "critical",
    recommendation: "Potential SQL injection attack detected. Never concatenate directly into SQL queries.",
    patterns: [
      /('|"|;)\s*(drop|delete|update|insert|truncate|alter)\s+/i,
      /\bor\s+['"]?\d+['"]?\s*=\s*['"]?\d+/i,
      /\band\s+['"]?\d+['"]?\s*=\s*['"]?\d+/i,
      /union\s+(all\s+)?select\s+/i,
      /--\s/,
      /;\s*drop\s+table/i,
      /;\s*delete\s+from/i,
      /information_schema/i,
      /xp_cmdshell|exec_sp|sp_executesql/i,
    ],
  },
  {
    type: "command_injection",
    severity: "critical",
    recommendation: "Potential command injection detected. Never concatenate directly into shell commands.",
    patterns: [
      /[;&|`$(){}!<>].*(?:rm|del|move|copy|shutdown|reboot|chmod|chown)\s/i,
      /\$\(.*\)/,
      /`[^`]+`/,
      /&&\s*(rm|del|format|shutdown|reboot)/i,
      /\|\|\s*(rm|del|format|shutdown|reboot)/i,
      /;\s*(rm|del|format|shutdown|reboot)\s/i,
      /\|\s*(rm|del|format|shutdown|reboot)\s/i,
    ],
  },
  {
    type: "path_traversal",
    severity: "high",
    recommendation: "Path traversal attempt detected. Ensure file operations are within allowed directories.",
    patterns: [
      /\.\.\//g,
      /\.\.\\/g,
      /%2e%2e%2f/i,
      /%2e%2e\//i,
      /\.\.%2f/i,
      /etc\/passwd|etc\/shadow|proc\/self/i,
      /\.\.\/\.\.\/\.\.\//i,
    ],
  },
  {
    type: "xss",
    severity: "high",
    recommendation: "Potential XSS script detected. HTML escape must be applied before output.",
    patterns: [
      /<script[\s>]/i,
      /<\/script>/i,
      /javascript\s*:/i,
      /on\w+\s*=\s*["'][^"']*["']/i,
      /<iframe[\s>]/i,
      /<img[^>]+onerror/i,
      /<svg[\s>]*on\w+/i,
      /<video[^>]+on\w+/i,
    ],
  },
  {
    type: "exfiltration",
    severity: "medium",
    recommendation: "Potential data exfiltration intent detected.",
    patterns: [
      /(?:send|export|exfiltrate|leak|dump)\s+(?:my|the|all)\s+(?:data|password|secret|key|token|credential)/i,
      /\b(?:api|webhook|url|endpoint)[^\s]*\b.*(?:send|post|upload|fetch)\b/i,
      /(?:pastebin|gist|github\.com\/|bit\.ly)/i,
    ],
  },
  {
    type: "role_play_attack",
    severity: "medium",
    recommendation: "Role impersonation attack detected.",
    patterns: [
      /act\s+as\s+(?:a|an|the)\s+(?:system|admin|root|developer|attacker|hacker)/i,
      /you\s+are\s+(?:now\s+)?(?:a|an|the)\s+(?:system|admin|root|god)/i,
      /developer\s+mode|god\s+mode|admin\s+mode/i,
      /DAN\s*(?:mode)?|do\s+anything\s+now/i,
    ],
  },
  {
    type: "jailbreak",
    severity: "high",
    recommendation: "Jailbreak attempt detected.",
    patterns: [
      /jailbreak|crack|hack|exploit/i,
      /bypass\s+(safety|security|filter|guard)/i,
      /ignore\s+(content|safety|security)\s+policy/i,
      /this\s+is\s+(?:a|an)\s+(?:fiction|roleplay|simulation|test)/i,
      /(?:hypothetical|fictional|pretend)\s+(?:scenario|situation|world)/i,
      /(?:for|in)\s+(?:educational|educational|academic)\s+(?:purposes?|use|reason)/i,
    ],
  },
];

/** Injection detector */
export class InjectionDetector {
  private rules: DetectionRule[];
  private customRules: DetectionRule[] = [];

  constructor(customRules?: DetectionRule[]) {
    this.rules = [...DETECTION_RULES, ...(customRules || [])];
  }

  /** Add custom rule */
  addRule(rule: DetectionRule): void {
    this.customRules.push(rule);
    this.rules = [...DETECTION_RULES, ...this.customRules];
  }

  /** Detect injection attacks in input */
  detect(input: string): InjectionDetection[] {
    const detections: InjectionDetection[] = [];

    for (const rule of this.rules) {
      for (const pattern of rule.patterns) {
        const match = pattern.exec(input);
        if (match) {
          detections.push({
            type: rule.type,
            severity: rule.severity,
            matchedPattern: match[0],
            position: match.index,
            context: input.slice(
              Math.max(0, match.index - 20),
              Math.min(input.length, match.index + match[0].length + 20),
            ),
            confidence: this.calculateConfidence(rule.type, match[0]),
            recommendation: rule.recommendation,
          });
        }
      }
    }

    return detections.sort((a, b) => {
      const severityOrder = { critical: 0, high: 1, medium: 2, low: 3 };
      return severityOrder[a.severity] - severityOrder[b.severity];
    });
  }

  /** Full sanitization flow */
  sanitize(input: string): SanitizationResult {
    const detections = this.detect(input);

    if (detections.length === 0) {
      return {
        isClean: true,
        sanitizedText: input,
        detections: [],
        riskScore: 0,
      };
    }

    const riskScore = this.calculateRiskScore(detections);
    const sanitizedText = this.redactDetections(input, detections);

    return {
      isClean: false,
      sanitizedText,
      detections,
      riskScore,
    };
  }

  /** Detect only critical and high severity */
  detectCritical(input: string): InjectionDetection[] {
    return this.detect(input).filter(
      (d) => d.severity === "critical" || d.severity === "high",
    );
  }

  /** Calculate confidence score */
  private calculateConfidence(type: InjectionType, matched: string): number {
    const baseScores: Record<InjectionType, number> = {
      prompt_injection: 0.85,
      sql_injection: 0.95,
      command_injection: 0.9,
      path_traversal: 0.85,
      xss: 0.8,
      exfiltration: 0.6,
      role_play_attack: 0.7,
      jailbreak: 0.75,
    };

    const score = baseScores[type] || 0.5;
    const lengthBonus = Math.min(matched.length / 30, 0.1);
    return Math.min(score + lengthBonus, 1.0);
  }

  /** Calculate risk score */
  private calculateRiskScore(detections: InjectionDetection[]): number {
    const severityWeights = { critical: 1, high: 0.75, medium: 0.5, low: 0.25 };
    const total = detections.reduce((sum, d) => {
      return sum + (severityWeights[d.severity] || 0.25) * d.confidence;
    }, 0);
    return Math.min(total / Math.max(detections.length, 1), 1.0);
  }

  /** Redact detected injection content */
  private redactDetections(
    input: string,
    detections: InjectionDetection[],
  ): string {
    let result = input;
    const sorted = [...detections].sort((a, b) => b.position - a.position);

    for (const detection of sorted) {
      const start = detection.position;
      const end = start + detection.matchedPattern.length;
      result = result.slice(0, start) + `[FILTERED: ${detection.type}]` + result.slice(end);
    }

    return result;
  }
}

/** HTML escape */
export function escapeHtml(text: string): string {
  const div = document.createElement("div");
  div.textContent = text;
  return div.innerHTML;
}

/** JSON safe stringify */
export function safeStringify(value: unknown): string {
  try {
    return JSON.stringify(value, (_key, val) => {
      if (typeof val === "string") {
        if (/<script|javascript:|on\w+=/i.test(val)) {
          return "[FILTERED]";
        }
      }
      return val;
    });
  } catch {
    return "null";
  }
}

let _detector: InjectionDetector | null = null;
export function getInjectionDetector(): InjectionDetector {
  if (!_detector) {
    _detector = new InjectionDetector();
  }
  return _detector;
}
