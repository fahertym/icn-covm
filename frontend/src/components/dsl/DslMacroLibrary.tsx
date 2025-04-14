'use client';

import { useState, useEffect } from 'react';
import { dslApi } from '@/lib/api';
import { MacroInfo } from '@/lib/api';

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

export default function DslMacroLibrary({ onSelectMacro }: DslMacroLibraryProps) {
  const [macros, setMacros] = useState<MacroItem[]>([]);
  const [loading, setLoading] = useState<boolean>(true);
  const [error, setError] = useState<string | null>(null);
  const [searchQuery, setSearchQuery] = useState<string>('');
  const [selectedCategory, setSelectedCategory] = useState<string>('all');

  // Load macros from API
  useEffect(() => {
    setLoading(true);
    setError(null);
    
    dslApi.listMacros()
      .then(macroList => {
        // Convert API response to component format
        const formattedMacros = macroList.map((macro: MacroInfo) => ({
          id: macro.id,
          name: macro.name,
          description: macro.description || 'No description provided',
          category: macro.category || 'uncategorized',
          lastUpdated: macro.updated_at,
        }));
        
        setMacros(formattedMacros);
        setLoading(false);
      })
      .catch(err => {
        console.error("Failed to load macros:", err);
        setError("Failed to load macro library. Please try again.");
        setLoading(false);
      });
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

  // Handle macro deletion
  const handleDeleteMacro = (macroId: string, event: React.MouseEvent<HTMLButtonElement>) => {
    event.stopPropagation(); // Prevent selecting the macro when clicking delete
    
    if (confirm('Are you sure you want to delete this macro?')) {
      setLoading(true);
      
      dslApi.deleteMacro(macroId)
        .then(() => {
          // Remove the deleted macro from the state
          setMacros(macros.filter(macro => macro.id !== macroId));
          setLoading(false);
        })
        .catch(err => {
          console.error("Failed to delete macro:", err);
          setError("Failed to delete macro. Please try again.");
          setLoading(false);
        });
    }
  };

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
        ) : error ? (
          <div className="text-center py-8 text-red-500">{error}</div>
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
                  <div className="flex gap-2">
                    <span className="text-xs px-2 py-1 bg-gray-100 rounded-full">
                      {macro.category}
                    </span>
                    <button
                      className="text-red-500 hover:text-red-700"
                      onClick={(e) => handleDeleteMacro(macro.id, e)}
                      aria-label="Delete macro"
                    >
                      <svg xmlns="http://www.w3.org/2000/svg" className="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                      </svg>
                    </button>
                  </div>
                </div>
                <p className="text-sm text-gray-600 mt-2">{macro.description}</p>
                <div className="text-xs text-gray-500 mt-3">
                  Last updated: {new Date(macro.lastUpdated).toLocaleDateString()}
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