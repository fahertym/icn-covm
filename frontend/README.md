# ICN-COVM Visual DSL Interface

This is the frontend for the ICN-COVM (Inter-Cooperative Network - Cooperative Virtual Machine) project, providing a visual interface for working with the DSL (Domain Specific Language) and governance features.

## Features

- **Visual DSL Editor**: Create and edit DSL macros with a visual, drag-and-drop interface
- **Macro Library**: Browse, search, and manage existing macros
- **Code View**: View and edit the raw DSL code
- **Proposal Visualization**: Visualize governance proposal workflows
- **API Integration**: Connects to the ICN-COVM Rust backend

## Technologies

- **Next.js**: React framework with server-side rendering
- **React Flow**: For the interactive visual DSL editor
- **Tailwind CSS**: For styling
- **Axios**: For API communication

## Getting Started

### Prerequisites

- Node.js 16+ and npm/yarn
- ICN-COVM backend running (typically on port 3030)

### Installation

1. Install dependencies:
   ```bash
   npm install
   ```

2. Start the development server:
   ```bash
   npm run dev
   ```

3. Open [http://localhost:3000](http://localhost:3000) in your browser

## Project Structure

- `src/app/*`: Next.js app router pages
- `src/components/*`: Reusable React components
- `src/components/dsl/*`: DSL editor components
- `src/lib/*`: Utility functions and API clients

## Visual DSL Editor Usage

1. Navigate to the `/dsl` route
2. Choose from:
   - **Visual Editor**: Drag and drop nodes to create flows
   - **Code View**: See and edit the raw DSL code
   - **Macro Library**: Browse existing macros

## Development

When developing, all API requests to `/api/*` are proxied to the backend server (configured in `next.config.js`).

To build for production:

```bash
npm run build
```

## License

ISC License 