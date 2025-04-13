export default function Home() {
  return (
    <div className="flex flex-col gap-6">
      <section className="text-center py-12">
        <h1 className="text-4xl font-bold mb-4">ICN-COVM Visual Interface</h1>
        <p className="text-xl max-w-3xl mx-auto text-gray-600">
          A visual interface for interacting with ICN-COVM's Domain Specific Language and governance processes.
        </p>
      </section>

      <div className="grid md:grid-cols-2 lg:grid-cols-3 gap-6">
        <div className="card">
          <h2 className="text-2xl font-bold mb-3 text-primary">Visual DSL Editor</h2>
          <p className="mb-4 text-gray-600">Create and edit DSL macros with an intuitive visual interface.</p>
          <a href="/dsl" className="btn btn-primary inline-block">Open Editor</a>
        </div>

        <div className="card">
          <h2 className="text-2xl font-bold mb-3 text-secondary">Governance</h2>
          <p className="mb-4 text-gray-600">Participate in governance processes and view current state.</p>
          <a href="/governance" className="btn btn-secondary inline-block">View Governance</a>
        </div>

        <div className="card">
          <h2 className="text-2xl font-bold mb-3 text-accent">Proposals</h2>
          <p className="mb-4 text-gray-600">Browse, create, and vote on active proposals.</p>
          <a href="/proposals" className="btn bg-accent text-white hover:bg-purple-600 inline-block">View Proposals</a>
        </div>
      </div>

      <section className="mt-8">
        <h2 className="text-2xl font-bold mb-4">Getting Started</h2>
        <div className="card">
          <ol className="list-decimal list-inside space-y-3">
            <li>Explore the Visual DSL editor to create and edit macros</li>
            <li>Browse existing proposals and their execution flows</li>
            <li>Visualize governance processes and decision-making</li>
            <li>Create new proposals with visual macro support</li>
          </ol>
        </div>
      </section>
    </div>
  );
} 