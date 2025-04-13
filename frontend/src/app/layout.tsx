import type { Metadata } from 'next';
import './globals.css';

export const metadata: Metadata = {
  title: 'ICN-COVM - Visual DSL Interface',
  description: 'A visual interface for the ICN-COVM Distributed State Machine',
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en">
      <body className="min-h-screen bg-background text-foreground">
        <div className="flex flex-col min-h-screen">
          <header className="bg-primary text-white p-4 shadow-md">
            <div className="container mx-auto flex justify-between items-center">
              <h1 className="text-2xl font-bold">ICN-COVM</h1>
              <nav>
                <ul className="flex space-x-6">
                  <li><a href="/" className="hover:text-accent transition-colors">Home</a></li>
                  <li><a href="/governance" className="hover:text-accent transition-colors">Governance</a></li>
                  <li><a href="/dsl" className="hover:text-accent transition-colors">Visual DSL</a></li>
                  <li><a href="/proposals" className="hover:text-accent transition-colors">Proposals</a></li>
                </ul>
              </nav>
            </div>
          </header>
          <main className="flex-grow container mx-auto p-6">{children}</main>
          <footer className="bg-gray-100 p-4 text-center text-gray-600">
            <div className="container mx-auto">
              <p>ICN-COVM © {new Date().getFullYear()} - Cooperative Virtual Machine</p>
            </div>
          </footer>
        </div>
      </body>
    </html>
  );
} 