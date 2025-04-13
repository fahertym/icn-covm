'use client';

import { memo } from 'react';
import { Handle, Position, NodeProps } from 'reactflow';

// Interface for the data passed to the DslNode
interface DslNodeData {
  label: string;
  value: string;
}

const DslNode = ({ data, isConnectable }: NodeProps<DslNodeData>) => {
  return (
    <div className="flow-node flow-node-dsl min-w-[120px]">
      <Handle
        type="target"
        position={Position.Top}
        isConnectable={isConnectable}
      />
      <div className="font-medium text-sm">{data.label}</div>
      <div className="text-xs bg-white px-2 py-1 rounded mt-1 border border-blue-200">
        {data.value}
      </div>
      <Handle
        type="source"
        position={Position.Bottom}
        isConnectable={isConnectable}
      />
    </div>
  );
};

export default memo(DslNode); 