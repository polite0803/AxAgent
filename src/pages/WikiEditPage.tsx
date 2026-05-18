import { WikiEditorPage } from "@/pages/WikiEditorPage";
import { useNavigate, useParams } from "react-router-dom";

/** Route-compatible wrapper — extracts noteId from URL params */
export function WikiEditPage() {
  const { noteId } = useParams<{ wikiId: string; noteId: string }>();
  const navigate = useNavigate();

  if (!noteId) {
    return null;
  }

  return (
    <WikiEditorPage
      noteId={noteId}
      onBack={() => navigate(-1)}
    />
  );
}
