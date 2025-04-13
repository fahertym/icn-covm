'use client';

import { memo } from 'react';
import { Handle, Position, NodeProps } from 'reactflow';

// Interface for the data passed to the MacroNode
interface MacroNodeData {
  label: string;
  params: string[];
}

const MacroNode = ({ data, isConnectable }: NodeProps<MacroNodeData>) => {
  return (
    <div className="flow-node flow-node-macro min-w-[150px]">
      <Handle
        type="target"
        position={Position.Top}
        isConnectable={isConnectable}
      />
      <div className="font-bold text-sm">{data.label}</div>
      {data.params && data.params.length > 0 && (
        <div className="flex flex-wrap gap-1 mt-2">
          {data.params.map((param, index) => (
            <span
              key={index}
              className="text-xs bg-white px-2 py-0.5 rounded border border-purple-200"
            >
              {param}
            </span>
          ))}
        </div>
      )}
      <Handle
        type="source"
        position={Position.Bottom}
        isConnectable={isConnectable}
      />
    </div>
  );
};

export default memo(MacroNode); 