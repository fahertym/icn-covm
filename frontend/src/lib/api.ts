import axios from 'axios';

// Create an Axios instance with baseURL and default headers
const api = axios.create({
  baseURL: '/api', // This will be proxied to the backend through Next.js
  headers: {
    'Content-Type': 'application/json',
  },
});

// Types for the API responses
export interface MacroInfo {
  id: string;
  name: string;
  description?: string;
  created_at: string;
  updated_at: string;
  category?: string;
}

export interface NodeData {
  [key: string]: any;
}

export interface NodeInfo {
  id: string;
  node_type: string;
  data: any;
  position: {
    x: number;
    y: number;
  };
}

export interface EdgeInfo {
  id: string;
  source: string;
  target: string;
  animated?: boolean;
  label?: string;
}

export interface MacroVisualRepresentation {
  nodes: NodeInfo[];
  edges: EdgeInfo[];
}

export interface MacroDetails {
  id: string;
  name: string;
  code: string;
  description?: string;
  created_at: string;
  updated_at: string;
  category?: string;
  visual_representation?: MacroVisualRepresentation;
}

export interface SaveMacroRequest {
  name: string;
  code: string;
  description?: string;
  category?: string;
  visual_representation?: MacroVisualRepresentation;
}

// API functions for the DSL macros
export const dslApi = {
  // Get a list of all macros
  async listMacros(): Promise<MacroInfo[]> {
    const response = await api.get<{ macros: MacroInfo[] }>('/dsl/macros');
    return response.data.macros;
  },

  // Get details of a specific macro
  async getMacro(id: string): Promise<MacroDetails> {
    const response = await api.get<MacroDetails>(`/dsl/macros/${id}`);
    return response.data;
  },

  // Save a macro (create or update)
  async saveMacro(macro: SaveMacroRequest): Promise<MacroDetails> {
    // If macro has an ID, update it, otherwise create a new one
    if ('id' in macro && macro.id) {
      const response = await api.put<MacroDetails>(`/dsl/macros/${macro.id}`, macro);
      return response.data;
    } else {
      const response = await api.post<MacroDetails>('/dsl/macros', macro);
      return response.data;
    }
  },

  // Delete a macro
  async deleteMacro(id: string): Promise<{ success: boolean, message: string }> {
    const response = await api.delete<{ success: boolean, message: string }>(`/dsl/macros/${id}`);
    return response.data;
  },

  // Execute a macro with parameters
  async executeMacro(id: string, params: any): Promise<any> {
    const response = await api.post<any>(`/dsl/macros/${id}/execute`, params);
    return response.data;
  }
};

// API functions for proposals
export const proposalApi = {
  // Get a specific proposal
  async getProposal(id: string) {
    const response = await api.get(`/proposals/${id}`);
    return response.data;
  },

  // Get comments for a proposal
  async getProposalComments(id: string, showHidden: boolean = false) {
    const response = await api.get(`/proposals/${id}/comments`, {
      params: { show_hidden: showHidden }
    });
    return response.data;
  },

  // Get summary for a proposal
  async getProposalSummary(id: string) {
    const response = await api.get(`/proposals/${id}/summary`);
    return response.data;
  },
};

export default api; 