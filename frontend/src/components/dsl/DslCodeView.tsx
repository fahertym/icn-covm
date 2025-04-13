'use client';

import { useState, useEffect } from 'react';

interface DslCodeViewProps {
  currentMacro: string | null;
}

export default function DslCodeView({ currentMacro }: DslCodeViewProps) {
  const [code, setCode] = useState<string>('');
  const [loading, setLoading] = useState<boolean>(false);

  useEffect(() => {
    if (currentMacro) {
      setLoading(true);
      // Simulated API call to fetch macro code
      setTimeout(() => {
        // This would be replaced with a real API call in production
        const sampleCode = `# ${currentMacro} macro definition
# This is a sample DSL code representation

Push 1.0
Store "balance"
EmitEvent "system" "Balance initialized"
Load "balance"
Push 10.0
Add
Store "balance"
EmitEvent "system" "Balance updated"
`;
        setCode(sampleCode);
        setLoading(false);
      }, 300);
    } else {
      setCode('// Select a macro to view its code');
    }
  }, [currentMacro]);

  return (
    <div className="h-full flex flex-col">
      <div className="bg-gray-100 p-3 border-b border-gray-300 flex justify-between items-center">
        <h3 className="font-medium">DSL Code View</h3>
        <div>
          <button className="btn bg-gray-200 hover:bg-gray-300 text-sm">
            Copy to Clipboard
          </button>
        </div>
      </div>
      <div className="flex-grow bg-gray-900 text-gray-100 p-4 font-mono text-sm overflow-auto">
        {loading ? (
          <div className="animate-pulse">Loading macro code...</div>
        ) : (
          <pre>{code}</pre>
        )}
      </div>
    </div>
  );
} 