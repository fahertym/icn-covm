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

  // Load nodes and edges from backend when currentMacro changes
  useEffect(() => {
    if (currentMacro) {
      // Here you would fetch the macro definition from your backend
      // For now, we're using the initial data
      console.log(`Loading macro: ${currentMacro}`);
      // Simulated API call
      // fetchMacro(currentMacro).then(data => {
      //   setNodes(data.nodes);
      //   setEdges(data.edges);
      // });
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

  return (
    <div className="h-full flex flex-col">
      <ControlPanel 
        onAddNode={handleAddNode}
        onDeleteNode={handleDeleteNode}
        onExportDsl={handleExportDsl}
        selectedNode={selectedNode}
        onUpdateNode={handleUpdateNode}
      />
      
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
    </div>
  );
} 