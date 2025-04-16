import { NavLink } from 'react-router-dom';

const LINKS = [
  { to: '/wallet', label: 'Wallet' },
  { to: '/issuer', label: 'Issuer' },
  { to: '/verifier', label: 'Verifier' },
] as const;

/** Primary site navigation. Uses a real `<nav>` with an accessible label. */
export function Nav() {
  return (
    <nav className="site-nav" aria-label="Primary">
      <div className="site-nav__inner">
        <NavLink to="/wallet" className="site-nav__brand">
          <span className="site-nav__mark" aria-hidden="true">
            S
          </span>
          <span>Sigil</span>
        </NavLink>
        <ul className="site-nav__links">
          {LINKS.map((link) => (
            <li key={link.to}>
              <NavLink to={link.to} className="site-nav__link">
                {link.label}
              </NavLink>
            </li>
          ))}
        </ul>
      </div>
    </nav>
  );
}
