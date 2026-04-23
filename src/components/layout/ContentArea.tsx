import { Routes, Route } from 'react-router-dom';
import { ChatPage } from '@/pages/ChatPage';
import { KnowledgePage } from '@/pages/KnowledgePage';
import { MemoryPage } from '@/pages/MemoryPage';
import { LinkPage } from '@/pages/LinkPage';
import { GatewayPage } from '@/pages/GatewayPage';
import { FilesPage } from '@/pages/FilesPage';
import { SettingsPage } from '@/pages/SettingsPage';
import { SkillsPage } from '@/pages/SkillsPage';
import { PageErrorBoundary } from '@/components/shared/ErrorBoundary';

function SafeChatPage() {
  return (
    <PageErrorBoundary title="Chat Error">
      <ChatPage />
    </PageErrorBoundary>
  );
}

function SafeKnowledgePage() {
  return (
    <PageErrorBoundary title="Knowledge Error">
      <KnowledgePage />
    </PageErrorBoundary>
  );
}

function SafeMemoryPage() {
  return (
    <PageErrorBoundary title="Memory Error">
      <MemoryPage />
    </PageErrorBoundary>
  );
}

function SafeLinkPage() {
  return (
    <PageErrorBoundary title="Link Error">
      <LinkPage />
    </PageErrorBoundary>
  );
}

function SafeGatewayPage() {
  return (
    <PageErrorBoundary title="Gateway Error">
      <GatewayPage />
    </PageErrorBoundary>
  );
}

function SafeFilesPage() {
  return (
    <PageErrorBoundary title="Files Error">
      <FilesPage />
    </PageErrorBoundary>
  );
}

function SafeSettingsPage() {
  return (
    <PageErrorBoundary title="Settings Error">
      <SettingsPage />
    </PageErrorBoundary>
  );
}

function SafeSkillsPage() {
  return (
    <PageErrorBoundary title="Skills Error">
      <SkillsPage />
    </PageErrorBoundary>
  );
}

export function ContentArea() {
  return (
    <Routes>
      <Route path="/" element={<SafeChatPage />} />
      <Route path="/knowledge" element={<SafeKnowledgePage />} />
      <Route path="/memory" element={<SafeMemoryPage />} />
      <Route path="/link" element={<SafeLinkPage />} />
      <Route path="/gateway" element={<SafeGatewayPage />} />
      <Route path="/files" element={<SafeFilesPage />} />
      <Route path="/settings/*" element={<SafeSettingsPage />} />
      <Route path="/skills" element={<SafeSkillsPage />} />
    </Routes>
  );
}
