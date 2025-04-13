'use client';

import { memo } from 'react';
import { Handle, Position, NodeProps } from 'reactflow';

// Interface for the data passed to the ActionNode
interface ActionNodeData {
  label: string;
  category: string;
  message: string;
}

const ActionNode = ({ data, isConnectable }: NodeProps<ActionNodeData>) => {
  return (
    <div className="flow-node flow-node-action min-w-[150px]">
      <Handle
        type="target"
        position={Position.Top}
        isConnectable={isConnectable}
      />
      <div className="font-medium text-sm">{data.label}</div>
      <div className="mt-1">
        <div className="text-xs bg-white px-2 py-0.5 rounded border border-green-200 mb-1">
          {data.category}
        </div>
        <div className="text-xs bg-white px-2 py-0.5 rounded border border-green-200 italic">
          "{data.message}"
        </div>
      </div>
      <Handle
        type="source"
        position={Position.Bottom}
        isConnectable={isConnectable}
      />
    </div>
  );
};

export default memo(ActionNode); 