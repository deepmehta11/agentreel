"use client";

import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";

/** Renders markdown with syntax highlighting and proper styling. */
export function Markdown({ content }: { content: string }) {
  return (
    <ReactMarkdown
      remarkPlugins={[remarkGfm]}
      components={{
        p: ({ children }) => <p className="mb-2 last:mb-0">{children}</p>,
        h1: ({ children }) => <h1 className="text-base font-bold mb-2 text-gray-100">{children}</h1>,
        h2: ({ children }) => <h2 className="text-sm font-bold mb-1.5 text-gray-100">{children}</h2>,
        h3: ({ children }) => <h3 className="text-sm font-semibold mb-1 text-gray-200">{children}</h3>,
        strong: ({ children }) => <strong className="text-white font-semibold">{children}</strong>,
        em: ({ children }) => <em className="text-gray-300 italic">{children}</em>,
        code: ({ className, children, ...props }) => {
          const isBlock = className?.includes("language-");
          if (isBlock) {
            return (
              <pre className="bg-gray-950 border border-gray-800 rounded p-3 my-2 overflow-x-auto">
                <code className="text-green-300 text-xs">{children}</code>
              </pre>
            );
          }
          return (
            <code className="bg-gray-800 text-brand-300 px-1 py-0.5 rounded text-xs" {...props}>
              {children}
            </code>
          );
        },
        pre: ({ children }) => <>{children}</>,
        ul: ({ children }) => <ul className="list-disc ml-4 mb-2 space-y-0.5">{children}</ul>,
        ol: ({ children }) => <ol className="list-decimal ml-4 mb-2 space-y-0.5">{children}</ol>,
        li: ({ children }) => <li className="text-gray-300">{children}</li>,
        a: ({ href, children }) => (
          <a href={href} className="text-brand-400 hover:text-brand-300 underline" target="_blank" rel="noopener noreferrer">
            {children}
          </a>
        ),
        table: ({ children }) => (
          <div className="overflow-x-auto my-2">
            <table className="text-xs border-collapse w-full">{children}</table>
          </div>
        ),
        thead: ({ children }) => <thead className="border-b border-gray-700">{children}</thead>,
        th: ({ children }) => <th className="text-left px-2 py-1 text-gray-400 font-medium">{children}</th>,
        td: ({ children }) => <td className="px-2 py-1 text-gray-300 border-b border-gray-800/50">{children}</td>,
        blockquote: ({ children }) => (
          <blockquote className="border-l-2 border-gray-700 pl-3 my-2 text-gray-400 italic">{children}</blockquote>
        ),
        hr: () => <hr className="border-gray-800 my-3" />,
      }}
    >
      {content}
    </ReactMarkdown>
  );
}
