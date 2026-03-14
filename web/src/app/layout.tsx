import type { Metadata } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title: "AgentReel",
  description: "Browse, fork, and compare AI agent runs",
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en">
      <body className="min-h-screen">
        <nav className="border-b border-gray-800 px-6 py-4">
          <div className="max-w-7xl mx-auto flex items-center justify-between">
            <a href="/" className="text-xl font-bold text-brand-400">
              AgentReel
            </a>
            <div className="flex gap-6 text-sm text-gray-400">
              <a href="/" className="hover:text-white">
                Browse
              </a>
              <a href="/upload" className="hover:text-white">
                Upload
              </a>
              <a href="/compare" className="hover:text-white">
                Compare
              </a>
            </div>
          </div>
        </nav>
        <main className="max-w-7xl mx-auto px-6 py-8">{children}</main>
      </body>
    </html>
  );
}
