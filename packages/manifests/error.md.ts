export type ErrorManifestSection = {
  id: string;
  title: string;
};

export const ERROR_MANIFEST_SECTIONS: readonly ErrorManifestSection[] = [
  { id: "error-chain", title: "Error Chain" },
  { id: "job-process-session-context", title: "Job / Process / Session Context" },
  { id: "runtime-context", title: "Runtime Context" },
  { id: "error-context", title: "Error Context" },
  { id: "linked-artifacts", title: "Linked Artifacts" },
  { id: "next-debugging-action", title: "Next Debugging Action" },
  { id: "resolution", title: "Resolution" }
] as const;

export const renderErrorMarkdown = (
  sectionBodies: ReadonlyMap<string, string>
): string => {
  return ERROR_MANIFEST_SECTIONS.map(({ id, title }) => {
    const body = sectionBodies.get(id)?.trim() ?? "_Section unavailable._";
    return `## ${title}\n\n${body}`;
  }).join("\n\n");
};
