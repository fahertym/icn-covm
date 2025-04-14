'use client';

import { useState } from 'react';
import { Node } from 'reactflow';

interface ControlPanelProps {
  onAddNode: (nodeType: string, data: any) => void;
  onDeleteNode: () => void;
  onExportDsl: () => void;
  selectedNode: Node | null;
  onUpdateNode: (nodeId: string, newData: any) => void;
  onSave?: () => void;
}

export default function ControlPanel({
  onAddNode,
  onDeleteNode,
  onExportDsl,
  selectedNode,
  onUpdateNode,
  onSave,
}: ControlPanelProps) {
  const [nodeType, setNodeType] = useState<string>('dslNode');
  const [nodeOperation, setNodeOperation] = useState<string>('Push');
  const [nodeValue, setNodeValue] = useState<string>('');
  const [nodeCategory, setNodeCategory] = useState<string>('system');
  const [nodeMessage, setNodeMessage] = useState<string>('');
  const [macroName, setMacroName] = useState<string>('');
  const [macroParams, setMacroParams] = useState<string>('');

  // Handle form submission for adding a new node
  const handleAddNode = (e: React.FormEvent) => {
    e.preventDefault();
    
    let nodeData = {};
    
    if (nodeType === 'dslNode') {
      nodeData = {
        label: nodeOperation,
        value: nodeValue,
      };
    } else if (nodeType === 'actionNode') {
      nodeData = {
        label: 'EmitEvent',
        category: nodeCategory,
        message: nodeMessage,
      };
    } else if (nodeType === 'macroNode') {
      nodeData = {
        label: macroName,
        params: macroParams.split(',').map(param => param.trim()).filter(Boolean),
      };
    }
    
    onAddNode(nodeType, nodeData);
    
    // Reset form
    setNodeValue('');
    setNodeMessage('');
    setMacroName('');
    setMacroParams('');
  };

  // Handle updating a selected node
  const handleUpdateSelectedNode = (e: React.FormEvent) => {
    e.preventDefault();
    
    if (!selectedNode) return;
    
    let updatedData = {};
    
    if (selectedNode.type === 'dslNode') {
      updatedData = {
        label: nodeOperation,
        value: nodeValue,
      };
    } else if (selectedNode.type === 'actionNode') {
      updatedData = {
        category: nodeCategory,
        message: nodeMessage,
      };
    } else if (selectedNode.type === 'macroNode') {
      updatedData = {
        label: macroName,
        params: macroParams.split(',').map(param => param.trim()).filter(Boolean),
      };
    }
    
    onUpdateNode(selectedNode.id, updatedData);
  };

  // Populate form when a node is selected
  const populateFormFromSelectedNode = () => {
    if (!selectedNode) return;
    
    if (selectedNode.type === 'dslNode') {
      setNodeType('dslNode');
      setNodeOperation(selectedNode.data.label);
      setNodeValue(selectedNode.data.value || '');
    } else if (selectedNode.type === 'actionNode') {
      setNodeType('actionNode');
      setNodeCategory(selectedNode.data.category || 'system');
      setNodeMessage(selectedNode.data.message || '');
    } else if (selectedNode.type === 'macroNode') {
      setNodeType('macroNode');
      setMacroName(selectedNode.data.label || '');
      setMacroParams(selectedNode.data.params?.join(', ') || '');
    }
  };

  // Update form when selectedNode changes
  if (selectedNode && selectedNode.id !== '') {
    populateFormFromSelectedNode();
  }

  return (
    <div className="bg-gray-100 p-4 border-b border-gray-300">
      <div className="flex flex-wrap gap-4">
        <div className="flex-grow">
          <form onSubmit={selectedNode ? handleUpdateSelectedNode : handleAddNode} className="flex gap-3 flex-wrap">
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-1">Node Type</label>
              <select
                value={nodeType}
                onChange={(e) => setNodeType(e.target.value)}
                className="p-2 border rounded w-full"
                disabled={!!selectedNode}
              >
                <option value="dslNode">Basic Operation</option>
                <option value="actionNode">Action</option>
                <option value="macroNode">Macro</option>
              </select>
            </div>

            {nodeType === 'dslNode' && (
              <>
                <div>
                  <label className="block text-sm font-medium text-gray-700 mb-1">Operation</label>
                  <select
                    value={nodeOperation}
                    onChange={(e) => setNodeOperation(e.target.value)}
                    className="p-2 border rounded w-full"
                  >
                    <option value="Push">Push</option>
                    <option value="Store">Store</option>
                    <option value="Load">Load</option>
                    <option value="Add">Add</option>
                    <option value="Sub">Subtract</option>
                    <option value="Mul">Multiply</option>
                    <option value="Div">Divide</option>
                  </select>
                </div>
                <div>
                  <label className="block text-sm font-medium text-gray-700 mb-1">Value</label>
                  <input
                    type="text"
                    value={nodeValue}
                    onChange={(e) => setNodeValue(e.target.value)}
                    className="p-2 border rounded w-full"
                    placeholder="Value or variable name"
                  />
                </div>
              </>
            )}

            {nodeType === 'actionNode' && (
              <>
                <div>
                  <label className="block text-sm font-medium text-gray-700 mb-1">Category</label>
                  <input
                    type="text"
                    value={nodeCategory}
                    onChange={(e) => setNodeCategory(e.target.value)}
                    className="p-2 border rounded w-full"
                    placeholder="Event category"
                  />
                </div>
                <div>
                  <label className="block text-sm font-medium text-gray-700 mb-1">Message</label>
                  <input
                    type="text"
                    value={nodeMessage}
                    onChange={(e) => setNodeMessage(e.target.value)}
                    className="p-2 border rounded w-full"
                    placeholder="Event message"
                  />
                </div>
              </>
            )}

            {nodeType === 'macroNode' && (
              <>
                <div>
                  <label className="block text-sm font-medium text-gray-700 mb-1">Macro Name</label>
                  <input
                    type="text"
                    value={macroName}
                    onChange={(e) => setMacroName(e.target.value)}
                    className="p-2 border rounded w-full"
                    placeholder="Macro name"
                  />
                </div>
                <div>
                  <label className="block text-sm font-medium text-gray-700 mb-1">Parameters (comma separated)</label>
                  <input
                    type="text"
                    value={macroParams}
                    onChange={(e) => setMacroParams(e.target.value)}
                    className="p-2 border rounded w-full"
                    placeholder="param1, param2, ..."
                  />
                </div>
              </>
            )}

            <div className="flex items-end">
              <button
                type="submit"
                className="btn btn-primary"
              >
                {selectedNode ? 'Update Node' : 'Add Node'}
              </button>
            </div>
          </form>
        </div>

        <div className="flex items-end space-x-2">
          {selectedNode && (
            <button
              onClick={onDeleteNode}
              className="btn bg-red-500 text-white hover:bg-red-600"
            >
              Delete Node
            </button>
          )}
          <button
            onClick={onExportDsl}
            className="btn bg-secondary text-white"
          >
            Generate Code
          </button>
          {onSave && (
            <button
              onClick={onSave}
              className="btn bg-primary text-white"
            >
              Save Macro
            </button>
          )}
        </div>
      </div>
    </div>
  );
} 