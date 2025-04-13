/** @type {import('next').NextConfig} */
const nextConfig = {
  reactStrictMode: true,
  swcMinify: true,
  async rewrites() {
    return [
      {
        source: '/api/:path*',
        destination: 'http://localhost:3030/api/:path*', // Proxy API requests to the Rust backend
      },
    ];
  },
};

module.exports = nextConfig; 