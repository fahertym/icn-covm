'use client';

import { useState, useEffect, useCallback } from 'react';
import ReactFlow, { 
  Background, 
  Controls, 
  MiniMap, 
  Node, 
  Edge, 
  NodeChange, 
  EdgeChange, 
  Connection, 
  addEdge, 
  applyNodeChanges, 
  applyEdgeChanges 
} from 'reactflow';
import 'reactflow/dist/style.css';
import { dslApi } from '@/lib/api';

import DslNode from './nodes/DslNode';
import MacroNode from './nodes/MacroNode';
import ActionNode from './nodes/ActionNode';
import ControlPanel from './ControlPanel';

// Define node types
const nodeTypes = {
  dslNode: DslNode,
  macroNode: MacroNode,
  actionNode: ActionNode
};

// Initial nodes for demonstration
const initialNodes: Node[] = [
  {
    id: '1',
    type: 'dslNode',
    data: { label: 'Push', value: '1.0' },
    position: { x: 250, y: 50 },
  },
  {
    id: '2',
    type: 'dslNode',
    data: { label: 'Store', value: 'balance' },
    position: { x: 250, y: 150 },
  },
  {
    id: '3',
    type: 'macroNode',
    data: { label: 'IncrementBalance', params: ['amount'] },
    position: { x: 50, y: 200 },
  },
  {
    id: '4',
    type: 'actionNode',
    data: { label: 'EmitEvent', category: 'system', message: 'Balance updated' },
    position: { x: 250, y: 250 },
  },
];

// Initial edges
const initialEdges: Edge[] = [
  {
    id: 'e1-2',
    source: '1',
    target: '2',
    animated: true,
  },
  {
    id: 'e2-4',
    source: '2',
    target: '4',
  },
  {
    id: 'e3-1',
    source: '3',
    target: '1',
    style: { stroke: '#8B5CF6' },
  },
];

interface DslVisualEditorProps {
  currentMacro: string | null;
  onMacroChange: (macroName: string | null) => void;
}

export default function DslVisualEditor({ currentMacro, onMacroChange }: DslVisualEditorProps) {
  const [nodes, setNodes] = useState<Node[]>(initialNodes);
  const [edges, setEdges] = useState<Edge[]>(initialEdges);
  const [selectedNode, setSelectedNode] = useState<Node | null>(null);
  const [loading, setLoading] = useState<boolean>(false);
  const [error, setError] = useState<string | null>(null);

  // Load nodes and edges from backend when currentMacro changes
  useEffect(() => {
    if (currentMacro) {
      setLoading(true);
      setError(null);
      
      // Fetch the macro definition from the API
      dslApi.getMacro(currentMacro)
        .then(macroData => {
          if (macroData.visual_representation) {
            // Convert the API data to ReactFlow format
            const apiNodes = macroData.visual_representation.nodes.map(node => ({
              id: node.id,
              type: node.node_type,
              data: node.data,
              position: node.position
            }));
            
            const apiEdges = macroData.visual_representation.edges.map(edge => ({
              id: edge.id,
              source: edge.source,
              target: edge.target,
              animated: edge.animated || false,
              label: edge.label
            }));
            
            setNodes(apiNodes);
            setEdges(apiEdges);
          } else {
            // No visual representation, reset to empty flow
            setNodes([]);
            setEdges([]);
          }
          setLoading(false);
        })
        .catch(err => {
          console.error("Failed to load macro:", err);
          setError("Failed to load macro. Please try again.");
          setLoading(false);
        });
    }
  }, [currentMacro]);

  // Handle node changes (position, selection)
  const onNodesChange = useCallback(
    (changes: NodeChange[]) => setNodes((nds) => applyNodeChanges(changes, nds)),
    []
  );

  // Handle edge changes
  const onEdgesChange = useCallback(
    (changes: EdgeChange[]) => setEdges((eds) => applyEdgeChanges(changes, eds)),
    []
  );

  // Handle new connections between nodes
  const onConnect = useCallback(
    (connection: Connection) => setEdges((eds) => addEdge(
      { ...connection, animated: false }, 
      eds
    )),
    []
  );

  // Handle node click for editing
  const onNodeClick = useCallback((event: React.MouseEvent, node: Node) => {
    setSelectedNode(node);
  }, []);

  // Add a new node to the flow
  const handleAddNode = (nodeType: string, data: any) => {
    const newNode: Node = {
      id: `node_${Date.now()}`,
      type: nodeType,
      data,
      position: { 
        x: Math.random() * 300 + 50, 
        y: Math.random() * 300 + 50 
      },
    };
    
    setNodes((nodes) => [...nodes, newNode]);
  };

  // Update node data
  const handleUpdateNode = (nodeId: string, newData: any) => {
    setNodes((nodes) => 
      nodes.map((node) => 
        node.id === nodeId 
          ? { ...node, data: { ...node.data, ...newData } } 
          : node
      )
    );
    setSelectedNode(null);
  };

  // Delete selected node
  const handleDeleteNode = () => {
    if (selectedNode) {
      setNodes((nodes) => nodes.filter((node) => node.id !== selectedNode.id));
      // Also remove any connected edges
      setEdges((edges) => 
        edges.filter(
          (edge) => edge.source !== selectedNode.id && edge.target !== selectedNode.id
        )
      );
      setSelectedNode(null);
    }
  };

  // Export the current flow as DSL code
  const handleExportDsl = () => {
    // Here you would convert the flow to DSL code
    // This is a simplified example
    const dslCode = nodes
      .sort((a, b) => 
        (a.position.y === b.position.y 
          ? a.position.x - b.position.x 
          : a.position.y - b.position.y)
      )
      .map(node => {
        if (node.type === 'dslNode') {
          if (node.data.label === 'Push') {
            return `Push ${node.data.value}`;
          } else if (node.data.label === 'Store') {
            return `Store "${node.data.value}"`;
          }
        } else if (node.type === 'actionNode') {
          if (node.data.label === 'EmitEvent') {
            return `EmitEvent "${node.data.category}" "${node.data.message}"`;
          }
        } else if (node.type === 'macroNode') {
          return `Macro "${node.data.label}"`;
        }
        return '';
      })
      .filter(line => line)
      .join('\n');
    
    console.log('Generated DSL code:');
    console.log(dslCode);
    
    // In a real implementation, you would send this to your backend
    // or update the code view
  };

  // Export both the DSL code and visual representation for saving
  const exportForSaving = () => {
    // Generate DSL code from nodes
    const dslCode = nodes
      .sort((a, b) => 
        (a.position.y === b.position.y 
          ? a.position.x - b.position.x 
          : a.position.y - b.position.y)
      )
      .map(node => {
        if (node.type === 'dslNode') {
          if (node.data.label === 'Push') {
            return `Push ${node.data.value}`;
          } else if (node.data.label === 'Store') {
            return `Store "${node.data.value}"`;
          } else if (node.data.label === 'Load') {
            return `Load "${node.data.value}"`;
          } else if (node.data.label === 'Add') {
            return 'Add';
          } else if (node.data.label === 'Sub') {
            return 'Sub';
          } else if (node.data.label === 'Mul') {
            return 'Mul';
          } else if (node.data.label === 'Div') {
            return 'Div';
          }
        } else if (node.type === 'actionNode') {
          if (node.data.label === 'EmitEvent') {
            return `EmitEvent "${node.data.category}" "${node.data.message}"`;
          }
        } else if (node.type === 'macroNode') {
          return `Macro "${node.data.label}"`;
        }
        return '';
      })
      .filter(line => line)
      .join('\n');
    
    // Create visual representation object
    const visualRepresentation = {
      nodes: nodes.map(node => ({
        id: node.id,
        node_type: node.type || 'dslNode',
        data: node.data,
        position: node.position
      })),
      edges: edges.map(edge => ({
        id: edge.id,
        source: edge.source,
        target: edge.target,
        animated: edge.animated,
        label: edge.label
      }))
    };
    
    return {
      code: dslCode,
      visualRepresentation
    };
  };

  return (
    <div className="h-full flex flex-col">
      <ControlPanel 
        onAddNode={handleAddNode}
        onDeleteNode={handleDeleteNode}
        onExportDsl={handleExportDsl}
        selectedNode={selectedNode}
        onUpdateNode={handleUpdateNode}
        onSave={() => {
          const { code, visualRepresentation } = exportForSaving();
          
          // Create a save request
          const saveRequest = {
            name: currentMacro || `New_Macro_${Date.now()}`,
            code: code,
            description: `DSL macro created with the visual editor`,
            category: "custom",
            visual_representation: visualRepresentation
          };
          
          // Send to API
          dslApi.saveMacro(saveRequest)
            .then(result => {
              console.log('Macro saved successfully:', result);
              // If this is a new macro, update the current macro name
              if (!currentMacro) {
                onMacroChange(result.name);
              }
            })
            .catch(err => {
              console.error('Failed to save macro:', err);
              setError('Failed to save macro. Please try again.');
            });
        }}
      />
      
      {loading && (
        <div className="flex-grow flex items-center justify-center bg-gray-100">
          <div className="text-lg text-gray-600">Loading macro...</div>
        </div>
      )}
      
      {error && (
        <div className="bg-red-100 border border-red-300 text-red-700 px-4 py-2 rounded">
          {error}
        </div>
      )}
      
      {!loading && (
        <div className="flex-grow">
          <ReactFlow
            nodes={nodes}
            edges={edges}
            onNodesChange={onNodesChange}
            onEdgesChange={onEdgesChange}
            onConnect={onConnect}
            onNodeClick={onNodeClick}
            nodeTypes={nodeTypes}
            fitView
          >
            <Background color="#aaa" gap={16} />
            <Controls />
            <MiniMap />
          </ReactFlow>
        </div>
      )}
    </div>
  );
} 