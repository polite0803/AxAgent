interface SkillIframePageProps {
  componentConfig: Record<string, unknown>;
}

export function SkillIframePage({ componentConfig }: SkillIframePageProps) {
  const url = (componentConfig.url as string) || "about:blank";

  return (
    <iframe
      src={url}
      title="Skill Iframe Page"
      sandbox="allow-scripts"
      style={{
        width: "100%",
        height: "100%",
        minHeight: 400,
        border: "none",
        backgroundColor: "#fff",
      }}
    />
  );
}
