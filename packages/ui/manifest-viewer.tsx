import type { ReactElement, ReactNode } from 'react';

export interface ManifestViewerProps {
  manifestMarkdown: string;
  manifestVersion: string;
  basisHash: string;
  generatedAt: string;
  sections?: string[];
  highlightBlockers?: boolean;
  onSectionClick?: (sectionId: string) => void;
}

function slugify(value: string): string {
  return value
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '');
}

function isSafeUrl(url: string): boolean {
  const normalized = url.trim().toLowerCase();
  return !(
    normalized.startsWith('javascript:') ||
    normalized.startsWith('data:') ||
    normalized.startsWith('file:')
  );
}

function renderInline(text: string): ReactNode[] {
  const nodes: ReactNode[] = [];
  const linkPattern = /\[([^\]]+)\]\(([^)]+)\)/g;
  let cursor = 0;

  for (const match of text.matchAll(linkPattern)) {
    const fullMatch = match[0];
    const label = match[1] ?? '';
    const target = match[2] ?? '';
    const start = match.index ?? 0;

    if (start > cursor) {
      nodes.push(text.slice(cursor, start));
    }

    if (isSafeUrl(target)) {
      nodes.push(
        <a href={target} rel="noreferrer" target="_blank">
          {label}
        </a>,
      );
    } else {
      nodes.push(label);
    }

    cursor = start + fullMatch.length;
  }

  if (cursor < text.length) {
    nodes.push(text.slice(cursor));
  }

  return nodes;
}

function extractSections(markdown: string): string[] {
  return markdown
    .split('\n')
    .map((line) => line.trim())
    .filter((line) => line.startsWith('#'))
    .map((line) => line.replace(/^#+\s*/, ''))
    .filter(Boolean);
}

function renderMarkdown(markdown: string, highlightBlockers: boolean): ReactNode[] {
  const lines = markdown.split('\n');
  const nodes: ReactNode[] = [];
  let listItems: ReactNode[] = [];

  const flushList = (): void => {
    if (listItems.length > 0) {
      nodes.push(<ul>{listItems}</ul>);
      listItems = [];
    }
  };

  for (const line of lines) {
    const trimmed = line.trim();
    if (trimmed === '') {
      flushList();
      continue;
    }

    if (trimmed.startsWith('#')) {
      flushList();
      const title = trimmed.replace(/^#+\s*/, '');
      const sectionId = slugify(title);
      const blocker = highlightBlockers && /blocker/i.test(title);
      nodes.push(
        <section data-section={sectionId}>
          <h2 id={sectionId}>{title}</h2>
          {blocker ? <span data-blocker="true">Blocker</span> : null}
          <button type="button" data-copy-id={sectionId}>
            Copy section id
          </button>
        </section>,
      );
      continue;
    }

    if (trimmed.startsWith('- ')) {
      listItems.push(<li>{renderInline(trimmed.slice(2))}</li>);
      continue;
    }

    flushList();
    nodes.push(<p>{renderInline(trimmed)}</p>);
  }

  flushList();
  return nodes;
}

export function ManifestViewer({
  manifestMarkdown,
  manifestVersion,
  basisHash,
  generatedAt,
  sections,
  highlightBlockers = false,
  onSectionClick,
}: ManifestViewerProps): ReactElement {
  const sectionHeadings = sections ?? extractSections(manifestMarkdown);

  return (
    <article className="manifest-viewer">
      <header>
        <p>Manifest version {manifestVersion}</p>
        <p>Basis hash {basisHash}</p>
        <p>Generated at {generatedAt}</p>
      </header>

      <nav aria-label="Manifest sections">
        <ul>
          {sectionHeadings.map((section) => {
            const sectionId = slugify(section);
            return (
              <li key={sectionId}>
                <button type="button" onClick={() => onSectionClick?.(sectionId)}>
                  {section}
                </button>
              </li>
            );
          })}
        </ul>
      </nav>

      <div className="manifest-viewer__content">
        {renderMarkdown(manifestMarkdown, highlightBlockers)}
      </div>
    </article>
  );
}
