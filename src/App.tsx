import { BrowserRouter, Navigate, Route, Routes } from "react-router-dom";
import { ConnectedPage } from "./pages/ConnectedPage/ConnectedPage";
import { LoginPage } from "./pages/LoginPage/LoginPage";
import { AppShell } from "./components/AppShell/AppShell";
import { AddWatchPage } from "./pages/AddWatchPage/AddWatchPage";
import { LanguageProvider } from "./components/LanguageProvider/LanguageProvider";

function App() {
  return (
    <LanguageProvider>
      <BrowserRouter>
        <Routes>
          <Route path="/" element={<Navigate to="/login" replace />} />
          <Route path="/login" element={<LoginPage />} />
          <Route element={<AppShell />}>
            <Route path="/connected" element={<ConnectedPage />} />
            <Route path="/watches/new" element={<AddWatchPage />} />
            <Route path="/watches/:watchId/edit" element={<AddWatchPage />} />
          </Route>
          <Route path="*" element={<Navigate to="/login" replace />} />
        </Routes>
      </BrowserRouter>
    </LanguageProvider>
  );
}

export default App;
