'use client';

import { useState, useEffect } from 'react';

interface MacroItem {
  id: string;
  name: string;
  description: string;
  category: string;
  lastUpdated: string;
}

interface DslMacroLibraryProps {
  onSelectMacro: (macroName: string) => void;
}

// Sample macro data for demonstration
const sampleMacros: MacroItem[] = [
  {
    id: '1',
    name: 'IncrementBalance',
    description: 'Increases the balance of an account by the specified amount',
    category: 'economic',
    lastUpdated: '2023-05-15',
  },
  {
    id: '2',
    name: 'CreateProposal',
    description: 'Creates a new governance proposal with specified parameters',
    category: 'governance',
    lastUpdated: '2023-06-02',
  },
  {
    id: '3',
    name: 'ValidateVoter',
    description: 'Validates that a voter has sufficient voting rights',
    category: 'governance',
    lastUpdated: '2023-06-10',
  },
  {
    id: '4',
    name: 'TransferTokens',
    description: 'Transfers tokens from one account to another',
    category: 'economic',
    lastUpdated: '2023-05-28',
  },
  {
    id: '5',
    name: 'RegisterIdentity',
    description: 'Registers a new identity in the system',
    category: 'identity',
    lastUpdated: '2023-05-30',
  },
];

export default function DslMacroLibrary({ onSelectMacro }: DslMacroLibraryProps) {
  const [macros, setMacros] = useState<MacroItem[]>([]);
  const [loading, setLoading] = useState<boolean>(true);
  const [searchQuery, setSearchQuery] = useState<string>('');
  const [selectedCategory, setSelectedCategory] = useState<string>('all');

  // Simulate loading macros from API
  useEffect(() => {
    // This would be an actual API call in production
    setTimeout(() => {
      setMacros(sampleMacros);
      setLoading(false);
    }, 500);
  }, []);

  // Filter macros based on search query and category
  const filteredMacros = macros.filter((macro) => {
    const matchesSearch = macro.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
                         macro.description.toLowerCase().includes(searchQuery.toLowerCase());
    const matchesCategory = selectedCategory === 'all' || macro.category === selectedCategory;
    return matchesSearch && matchesCategory;
  });

  // Get unique categories for the filter dropdown
  const categories = ['all', ...Array.from(new Set(macros.map((macro) => macro.category)))];

  return (
    <div className="h-full flex flex-col">
      <div className="bg-gray-100 p-3 border-b border-gray-300">
        <div className="flex flex-wrap gap-3">
          <div className="flex-grow">
            <input
              type="text"
              placeholder="Search macros..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="p-2 border rounded w-full"
            />
          </div>
          <div>
            <select
              value={selectedCategory}
              onChange={(e) => setSelectedCategory(e.target.value)}
              className="p-2 border rounded"
            >
              {categories.map((category) => (
                <option key={category} value={category}>
                  {category.charAt(0).toUpperCase() + category.slice(1)}
                </option>
              ))}
            </select>
          </div>
        </div>
      </div>

      <div className="flex-grow overflow-auto p-4">
        {loading ? (
          <div className="animate-pulse text-center py-8">Loading macros...</div>
        ) : filteredMacros.length === 0 ? (
          <div className="text-center py-8 text-gray-500">
            No macros found matching your criteria
          </div>
        ) : (
          <div className="grid md:grid-cols-2 gap-4">
            {filteredMacros.map((macro) => (
              <div
                key={macro.id}
                className="border rounded-lg p-4 hover:shadow-md transition-shadow cursor-pointer bg-white"
                onClick={() => onSelectMacro(macro.name)}
              >
                <div className="flex justify-between items-start">
                  <h3 className="font-bold text-primary">{macro.name}</h3>
                  <span className="text-xs px-2 py-1 bg-gray-100 rounded-full">
                    {macro.category}
                  </span>
                </div>
                <p className="text-sm text-gray-600 mt-2">{macro.description}</p>
                <div className="text-xs text-gray-500 mt-3">
                  Last updated: {macro.lastUpdated}
                </div>
              </div>
            ))}
          </div>
        )}
      </div>

      <div className="bg-gray-100 p-3 border-t border-gray-300 flex justify-between">
        <button
          className="btn bg-accent text-white"
          onClick={() => {
            // Create a new empty macro and immediately select it
            const newMacroName = `NewMacro_${Date.now()}`;
            onSelectMacro(newMacroName);
          }}
        >
          Create New Macro
        </button>
        <div className="text-sm text-gray-600">
          {filteredMacros.length} macros found
        </div>
      </div>
    </div>
  );
} 