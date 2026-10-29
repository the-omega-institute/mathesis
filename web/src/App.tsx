import { FeedbackPage } from './pages/Feedback';
import { BindingsPage, BindingPage, BindingProfilePage } from './pages/Bindings';
import { FormalizationsPage } from './pages/Formalizations';
import { useEffect } from 'react';
import { SupportersPage, StatementPage } from './pages/Contributions';
import { ConjecturePage } from './pages/Conjecture';
import { LeaderboardPage, EntrantPage } from './pages/Leaderboard';
import { Link } from './routing/router';
import { Header } from './components/Header';
import { BrandMark } from './components/BrandMark';
import { ConjecturesPage } from './pages/Conjectures';
import { ContributePage } from './pages/Contribute';
import { ContributorsPage } from './pages/Contributors';
import { HomePage } from './pages/Home';
import { NotFoundPage } from './pages/NotFound';
import { ResultPage } from './pages/Result';
import { PaperPage, PapersPage } from './pages/Papers';
import { PeoplePage } from './pages/People';
import { AuthorPage } from './pages/Author';
import { PolicyPage } from './pages/Policy';
import { SubmissionDetailPage } from './pages/SubmissionDetail';
import { SubmissionsPage } from './pages/Submissions';
import { SubmitPage } from './pages/Submit';
import { TaskPage, TasksPage } from './pages/Tasks';
import { useRouter } from './routing/router';
import { routePath, type Route } from './routing/routes';

function titleFor(route: Route): string {
  switch (route.kind) {
    case 'feedback':
      return 'Knowledge proposal';
    case 'binding_profile':
      return 'Binding contributions';
    case 'bindings':
      return 'Binding discussions';
    case 'binding':
      return 'Binding discussion';
    case 'supporters':
      return 'Supporters';
    case 'statement':
      return route.record;
    case 'home':
      return 'Mathesis';
    case 'leaderboard':
      return 'Leaderboard';
    case 'entrant':
      return 'Solver';
    case 'conjecture':
      return route.record;
    case 'formalizations':
      return 'Formalizations';
    case 'conjectures':
      return 'Conjectures';
    case 'papers':
      return 'Accepted papers';
    case 'result':
    case 'paper':
      return route.record;
    case 'policy':
      return 'Review policy';
    case 'submit':
      return 'Submit your work';
    case 'submissions':
      return 'My work';
    case 'queue':
      return 'Review queue';
    case 'submission':
      return 'Paper';
    case 'people':
      return 'People';
    case 'author':
      return route.name;
    case 'authors':
      return 'Authors';
    case 'contribute':
      return 'Contribute';
    case 'tasks':
      return 'Tasks';
    case 'task':
      return 'Task';
    case 'contributors':
      return 'Contributors';
    case 'not_found':
      return 'Not found';
  }
}

function Page({ route }: { route: Route }) {
  switch (route.kind) {
    case 'feedback':
      return <FeedbackPage key={routePath(route)} id={route.id} proposalKey={route.key} />;
    case 'binding_profile':
      return <BindingProfilePage key={route.person} person={route.person} />;
    case 'bindings':
      return <BindingsPage />;
    case 'binding':
      return <BindingPage key={routePath(route)} id={route.id} boardKey={route.key} />;
    case 'supporters':
      return <SupportersPage />;
    case 'statement':
      return <StatementPage record={route.record} claim={route.claim} />;
    case 'home':
      return <HomePage />;
    case 'leaderboard':
      return <LeaderboardPage />;
    case 'entrant':
      return <EntrantPage id={route.id} />;
    case 'conjecture':
      return <ConjecturePage record={route.record} claim={route.claim} />;
    case 'conjectures':
      return <ConjecturesPage />;
    case 'papers':
      return (
        <PapersPage
          key={routePath(route)}
          initialQuery={route.q}
          initialFilters={{ author: route.author, msc: route.msc, kind: route.work }}
        />
      );
    case 'result':
      return <ResultPage key={routePath(route)} record={route.record} claim={route.claim} />;
    case 'paper':
      return <PaperPage key={route.record} record={route.record} />;
    case 'policy':
      return <PolicyPage />;
    case 'formalizations':
      return <FormalizationsPage />;
    case 'submit':
      return (
        <SubmitPage
          key={routePath(route)}
          initialIntake={route.intake}
          targetRecord={route.record}
          targetClaim={route.claim}
        />
      );
    case 'submissions':
      return <SubmissionsPage key="mine" scope="mine" />;
    case 'queue':
      return <SubmissionsPage key="queue" scope="queue" />;
    case 'submission':
      return (
        <SubmissionDetailPage
          key={`${route.id}-${route.staff}`}
          id={route.id}
          staffView={route.staff}
        />
      );
    case 'people':
      return <PeoplePage />;
    case 'author':
      return <AuthorPage name={route.name} />;
    case 'authors':
      return <PapersPage />;
    case 'contribute':
      return <ContributePage />;
    case 'tasks':
      return (
        <TasksPage
          key={routePath(route)}
          submission={route.submission}
          initialKind={route.taskKind}
        />
      );
    case 'task':
      return <TaskPage key={route.id} id={route.id} />;
    case 'contributors':
      return <ContributorsPage />;
    case 'not_found':
      return <NotFoundPage path={route.path} />;
  }
}

export function App() {
  const { route } = useRouter();
  useEffect(() => {
    const title = titleFor(route);
    document.title = title === 'Mathesis' ? 'Mathesis' : `${title} · Mathesis`;
  }, [route]);
  return (
    <>
      <a className="skip-link" href="#main">
        Skip to content
      </a>
      <Header />
      <main id="main">
        <Page route={route} />
      </main>
      <footer className="site-footer">
        <div className="site-footer-inner">
          <div className="footer-identity">
            <Link to={{ kind: 'home' }} className="wordmark">
              <BrandMark />
              Mathesis
            </Link>
            <p>Mathematical research, review, and verified proofs.</p>
          </div>
          <nav aria-label="Footer">
            <Link to={{ kind: 'contribute' }}>Contribute</Link>
            <Link to={{ kind: 'leaderboard' }}>Leaderboard</Link>
            <Link to={{ kind: 'policy' }}>Policy</Link>
            <Link to={{ kind: 'supporters' }}>Supporters</Link>
          </nav>
          <span className="footer-note">Read research. Share results. Publish questions.</span>
        </div>
      </footer>
    </>
  );
}
