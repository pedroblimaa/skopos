import { BrowserRouter, Navigate, Route, Routes } from "react-router-dom";
import { ConnectedPage } from "./pages/ConnectedPage/ConnectedPage";
import { LoginPage } from "./pages/LoginPage/LoginPage";

function App() {
  return (
    <BrowserRouter>
      <Routes>
        <Route path="/" element={<Navigate to="/login" replace />} />
        <Route path="/login" element={<LoginPage />} />
        <Route path="/connected" element={<ConnectedPage />} />
        <Route path="*" element={<Navigate to="/login" replace />} />
      </Routes>
    </BrowserRouter>
  );
}

export default App;
