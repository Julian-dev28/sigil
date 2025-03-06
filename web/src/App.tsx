import { Navigate, Route, Routes } from 'react-router-dom';
import { Nav } from './components/Nav';
import { WalletPage } from './routes/WalletPage';
import { IssuerPage } from './routes/IssuerPage';
import { VerifierPage } from './routes/VerifierPage';

function NotFound() {
  return (
    <section aria-labelledby="notfound-heading" className="empty-state">
      <h1 id="notfound-heading">Page not found</h1>
      <p>The page you were looking for does not exist.</p>
    </section>
  );
}

/** App shell: skip link, navigation, routed main content, footer. */
export function App() {
  return (
    <div className="app-shell">
      <a className="skip-link" href="#main-content">
        Skip to main content
      </a>
      <Nav />
      <main id="main-content" className="app-main" tabIndex={-1}>
        <Routes>
          <Route path="/" element={<Navigate to="/wallet" replace />} />
          <Route path="/wallet" element={<WalletPage />} />
          <Route path="/issuer" element={<IssuerPage />} />
          <Route path="/verifier" element={<VerifierPage />} />
          <Route path="*" element={<NotFound />} />
        </Routes>
      </main>
      <footer className="site-footer">
        <p>
          Sigil — standards-based Verifiable Credentials. W3C VC Data Model 1.1,
          did:key / did:web, StatusList2021, EdDSA.
        </p>
      </footer>
    </div>
  );
}

export default App;
