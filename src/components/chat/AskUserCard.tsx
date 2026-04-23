import React, { useState, useMemo, useEffect } from 'react';
import { Card, Input, Button, Space, Typography, Radio, theme } from 'antd';
import { MessageCircleQuestion, CheckCircle2, Send, Loader2 } from 'lucide-react';
import { useAgentStore } from '@/stores';
import { useTranslation } from 'react-i18next';

const { Text } = Typography;
const { TextArea } = Input;

interface AskUserCardProps {
  askId: string;
  conversationId: string;
  question: string;
  options?: string[];
}

const MAX_CHARS = 500;

const AskUserCard: React.FC<AskUserCardProps> = ({ askId, question, options }) => {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const [answer, setAnswer] = useState('');
  const [selectedOption, setSelectedOption] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);
  const [submitted, setSubmitted] = useState(false);
  const [appeared, setAppeared] = useState(false);
  const respondAskUser = useAgentStore((s) => s.respondAskUser);

  useEffect(() => {
    const timer = setTimeout(() => setAppeared(true), 50);
    return () => clearTimeout(timer);
  }, []);

  const hasOptions = options && options.length > 0;
  const isSingleChoice = hasOptions && options.length <= 5;

  const canSubmit = useMemo(() => {
    if (hasOptions && isSingleChoice) return selectedOption !== null;
    return answer.trim().length > 0;
  }, [hasOptions, isSingleChoice, selectedOption, answer]);

  const handleSubmit = async () => {
    if (!canSubmit || submitting || submitted) return;

    let finalAnswer: string;
    if (hasOptions && isSingleChoice && selectedOption) {
      finalAnswer = selectedOption;
    } else {
      finalAnswer = answer.trim();
    }

    setSubmitting(true);
    try {
      await respondAskUser(askId, finalAnswer);
      setSubmitted(true);
    } catch {
      setSubmitting(false);
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey && !hasOptions) {
      e.preventDefault();
      handleSubmit();
    }
  };

  const questionLines = question.split('\n');
  const charCount = answer.length;
  const isNearLimit = charCount > MAX_CHARS * 0.8;

  const cardStyle: React.CSSProperties = {
    marginTop: 8,
    borderColor: submitted ? token.colorSuccessBorder : token.colorPrimary,
    opacity: appeared ? 1 : 0,
    transform: appeared ? 'translateY(0)' : 'translateY(-10px)',
    transition: 'all 0.3s ease-out',
  };

  return (
    <Card
      size="small"
      style={cardStyle}
      styles={{ body: { padding: '14px 16px' } }}
    >
      <Space orientation="vertical" style={{ width: '100%' }} size={12}>
        <Space size={10} align="start">
          <div
            style={{
              width: 32,
              height: 32,
              borderRadius: 8,
              backgroundColor: `${token.colorPrimary}15`,
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              flexShrink: 0,
            }}
          >
            <MessageCircleQuestion size={18} style={{ color: token.colorPrimary }} />
          </div>
          <div style={{ flex: 1 }}>
            <Text strong style={{ fontSize: 13, display: 'block', marginBottom: 4 }}>
              {t('agent.questionFromAgent', 'Question from Agent')}
            </Text>
            <Text style={{ whiteSpace: 'pre-wrap', fontSize: 14, lineHeight: 1.6 }}>
              {questionLines.map((line, i) => (
                <React.Fragment key={i}>
                  {i > 0 && <br />}
                  {line}
                </React.Fragment>
              ))}
            </Text>
          </div>
        </Space>

        {hasOptions && (
          <div
            style={{
              padding: '10px 12px',
              backgroundColor: token.colorBgLayout,
              borderRadius: 8,
              border: `1px solid ${token.colorBorderSecondary}`,
            }}
          >
            <Text type="secondary" style={{ fontSize: 11, display: 'block', marginBottom: 8 }}>
              {isSingleChoice
                ? t('agent.selectOneOption', 'Select one option:')
                : t('agent.selectOptions', 'Select applicable options:')}
            </Text>
            {isSingleChoice ? (
              <Radio.Group
                value={selectedOption}
                onChange={(e) => setSelectedOption(e.target.value)}
                disabled={submitting || submitted}
                style={{ width: '100%' }}
              >
                <Space direction="vertical" style={{ width: '100%' }} size={4}>
                  {options!.map((opt) => (
                    <Radio
                      key={opt}
                      value={opt}
                      style={{
                        padding: '6px 10px',
                        borderRadius: 6,
                        border: `1px solid ${selectedOption === opt ? token.colorPrimary : token.colorBorder}`,
                        backgroundColor: selectedOption === opt ? `${token.colorPrimary}10` : 'transparent',
                        transition: 'all 0.2s',
                      }}
                    >
                      <Text style={{ marginLeft: 6 }}>{opt}</Text>
                    </Radio>
                  ))}
                </Space>
              </Radio.Group>
            ) : (
              <Space direction="vertical" style={{ width: '100%' }} size={4}>
                {options!.map((opt) => (
                  <Button
                    key={opt}
                    type={selectedOption === opt ? 'primary' : 'default'}
                    size="small"
                    onClick={() => setSelectedOption(opt)}
                    disabled={submitting || submitted}
                    block
                    style={{
                      justifyContent: 'flex-start',
                      textAlign: 'left',
                      borderRadius: 6,
                    }}
                  >
                    {opt}
                  </Button>
                ))}
              </Space>
            )}
          </div>
        )}

        {!hasOptions || !isSingleChoice ? (
          <div>
            <TextArea
              value={answer}
              onChange={(e) => setAnswer(e.target.value.slice(0, MAX_CHARS))}
              placeholder={
                hasOptions
                  ? t('agent.supplementPlaceholder', 'Add additional context (optional)...')
                  : t('agent.typeAnswerPlaceholder', 'Type your answer...')
              }
              autoSize={{ minRows: 2, maxRows: 5 }}
              disabled={submitting || submitted}
              onKeyDown={handleKeyDown}
              style={{
                borderRadius: 8,
                fontSize: 14,
              }}
            />
            <div style={{ display: 'flex', justifyContent: 'flex-end', marginTop: 4 }}>
              <Text
                type={isNearLimit ? 'warning' : 'secondary'}
                style={{ fontSize: 11 }}
              >
                {charCount}/{MAX_CHARS}
              </Text>
            </div>
          </div>
        ) : null}

        <div style={{ display: 'flex', justifyContent: 'flex-end', alignItems: 'center', gap: 8 }}>
          {submitted ? (
            <Space size={6} style={{ color: token.colorSuccess }}>
              <CheckCircle2 size={16} />
              <Text style={{ color: token.colorSuccess, fontSize: 13 }}>
                {t('agent.answerSubmitted', 'Answer submitted')}
              </Text>
            </Space>
          ) : (
            <>
              <Text type="secondary" style={{ fontSize: 12 }}>
                {hasOptions && isSingleChoice && !selectedOption
                  ? t('agent.pleaseSelectOption', 'Please select an option')
                  : hasOptions && !isSingleChoice && !answer.trim()
                  ? t('agent.pleaseEnterAnswer', 'Please enter an answer')
                  : ''}
              </Text>
              <Button
                type="primary"
                size="small"
                onClick={handleSubmit}
                loading={submitting}
                disabled={!canSubmit}
                icon={<Send size={14} />}
                style={{ borderRadius: 6 }}
              >
                {t('agent.submitAnswer', 'Submit')}
              </Button>
            </>
          )}
        </div>

        {submitting && (
          <div style={{ display: 'flex', alignItems: 'center', gap: 6, color: token.colorPrimary }}>
            <Loader2 size={14} className="spin" />
            <Text type="secondary" style={{ fontSize: 12 }}>
              {t('agent.submitting', 'Submitting...')}
            </Text>
          </div>
        )}
      </Space>

      <style>{`
        .spin {
          animation: spin 1s linear infinite;
        }
        @keyframes spin {
          from { transform: rotate(0deg); }
          to { transform: rotate(360deg); }
        }
      `}</style>
    </Card>
  );
};

export default AskUserCard;