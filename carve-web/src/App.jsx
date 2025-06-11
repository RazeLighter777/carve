import React, { useState, useEffect } from 'react';
import VueMount from './VueMount';
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

function App() {
  const [user, setUser] = useState(null);
  const [score, setScore] = useState(null);
  const [announcement, setAnnouncement] = useState('');

  useEffect(() => {
    fetchApi('user').then(setUser);
    fetchApi('score').then(setScore);
    fetchApi('announcement').then(r => setAnnouncement(r.text));
  }, []);

  return (
    <div style={darkBg}>
      <header className="header">
        <h1>CARVE Competition Portal</h1>
        <div className="user-info">
          {user ? `Welcome, ${user.name}` : 'Loading...'}
        </div>
      </header>
      <main className="main-content">
        <section className="scoreboard">
          <h2>Scoreboard</h2>
          <div className="score">{score ? score.value : 'Loading...'}</div>
        </section>
        <section className="announcement">
          <h2>Announcement</h2>
          <div className="announcement-text">{announcement || 'No announcements.'}</div>
        </section>
        <section className="vue-section">
          <h2>Vue Component Demo</h2>
          <VueMount />
        </section>
      </main>
      <footer className="footer">
        &copy; {new Date().getFullYear()} CARVE. All rights reserved.
      </footer>
    </div>
  );
}

export default App;
