'use client';

import { useState } from 'react';
import DslVisualEditor from '@/components/dsl/DslVisualEditor';
import DslCodeView from '@/components/dsl/DslCodeView';
import DslMacroLibrary from '@/components/dsl/DslMacroLibrary';

export default function DslPage() {
  const [activeTab, setActiveTab] = useState<'editor' | 'code' | 'library'>('editor');
  const [currentMacro, setCurrentMacro] = useState<string | null>(null);

  return (
    <div className="flex flex-col gap-6">
      <div className="flex justify-between items-center">
        <h1 className="text-3xl font-bold">Visual DSL Editor</h1>
        <div className="flex gap-2">
          <button 
            className={`btn ${activeTab === 'editor' ? 'btn-primary' : 'bg-gray-200 hover:bg-gray-300'}`}
            onClick={() => setActiveTab('editor')}
          >
            Visual Editor
          </button>
          <button 
            className={`btn ${activeTab === 'code' ? 'btn-primary' : 'bg-gray-200 hover:bg-gray-300'}`}
            onClick={() => setActiveTab('code')}
          >
            Code View
          </button>
          <button 
            className={`btn ${activeTab === 'library' ? 'btn-primary' : 'bg-gray-200 hover:bg-gray-300'}`}
            onClick={() => setActiveTab('library')}
          >
            Macro Library
          </button>
        </div>
      </div>

      <div className="card h-[calc(100vh-16rem)]">
        {activeTab === 'editor' && (
          <DslVisualEditor 
            currentMacro={currentMacro} 
            onMacroChange={(macroName) => setCurrentMacro(macroName)} 
          />
        )}
        {activeTab === 'code' && (
          <DslCodeView 
            currentMacro={currentMacro} 
          />
        )}
        {activeTab === 'library' && (
          <DslMacroLibrary 
            onSelectMacro={(macroName) => {
              setCurrentMacro(macroName);
              setActiveTab('editor');
            }} 
          />
        )}
      </div>

      <div className="flex justify-between">
        <div>
          {currentMacro && (
            <span className="text-gray-600">
              Currently editing: <span className="font-semibold">{currentMacro}</span>
            </span>
          )}
        </div>
        <div className="flex gap-2">
          <button className="btn bg-gray-200 hover:bg-gray-300">
            Save Draft
          </button>
          <button className="btn btn-primary">
            Save Macro
          </button>
        </div>
      </div>
    </div>
  );
} 