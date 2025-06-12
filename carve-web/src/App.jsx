import React, { useState, useEffect } from 'react';
import './App.css';

const darkBg = {
  background: '#181a20',
  minHeight: '100vh',
  color: '#f3f4f6',
  fontFamily: 'Inter, sans-serif',
};

function fetchApi(path) {
  return fetch(`/api/v1/${path}`).then(r => r.json());
}

function Header({ user, setScoreboardOnly, scoreboardOnly }) {
  return (
    <header className="header">
      <h1>CARVE Competition Portal</h1>
      <nav className="nav-links">
        <a href="#" onClick={e => { e.preventDefault(); setScoreboardOnly(false); }}>Home</a>
        <span className="nav-sep">|</span>
        <a href="#" onClick={e => { e.preventDefault(); setScoreboardOnly(true); }}>Scoreboard</a>
      </nav>
      <div className="user-info">
        {user ? `Welcome, ${user.name}` : 'Loading...'}
      </div>
    </header>
  );
}

function App() {
  const [user, setUser] = useState(null);
  const [score, setScore] = useState(null);
  const [announcement, setAnnouncement] = useState('');
  const [scoreboardOnly, setScoreboardOnly] = useState(false);

  useEffect(() => {
    fetchApi('user').then(setUser);
    fetchApi('score').then(setScore);
    fetchApi('announcement').then(r => setAnnouncement(r.text));
  }, []);

  return (
    <div style={darkBg}>
      <Header user={user} setScoreboardOnly={setScoreboardOnly} scoreboardOnly={scoreboardOnly} />
      <main className="main-content">
        {scoreboardOnly ? (
          <section className="scoreboard expanded">
            <h2>Scoreboard</h2>
            <div className="score">{score ? score.value : 'Loading...'}</div>
            <button className="back-btn" onClick={() => setScoreboardOnly(false)} style={{marginTop: '2rem'}}>Back</button>
          </section>
        ) : (
          <>
            <section className="scoreboard" onClick={() => setScoreboardOnly(true)} style={{cursor: 'pointer'}}>
              <h2>Scoreboard</h2>
              <div className="score">{score ? score.value : 'Loading...'}</div>
            </section>
            <section className="announcement">
              <h2>Announcement</h2>
              <div className="announcement-text">{announcement || 'No announcements.'}</div>
            </section>
          </>
        )}
      </main>
      <footer className="footer">
        Licensed under the AGPL v3.
      </footer>
    </div>
  );
}

export default App;
