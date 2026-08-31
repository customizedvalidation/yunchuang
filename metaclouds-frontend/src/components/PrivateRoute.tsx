import React, { useState, useEffect } from 'react';
import { Navigate, Outlet } from 'react-router-dom';
import LayoutComponent from './Layout';
import { Spin } from 'antd';

const PrivateRoute: React.FC = () => {
  const [loading, setLoading] = useState(true);
  const [isAuthenticated, setIsAuthenticated] = useState(false);

  useEffect(() => {
    const token = localStorage.getItem('token');
    
    if (!token) {
      setIsAuthenticated(false);
      setLoading(false);
      return;
    }

    const payload = token.split('.')[1];
    try {
      const decoded = JSON.parse(atob(payload));
      const exp = decoded.exp * 1000;
      if (Date.now() > exp) {
        localStorage.removeItem('token');
        localStorage.removeItem('user');
        setIsAuthenticated(false);
      } else {
        setIsAuthenticated(true);
      }
    } catch {
      localStorage.removeItem('token');
      localStorage.removeItem('user');
      setIsAuthenticated(false);
    }
    
    setLoading(false);
  }, []);

  if (loading) {
    return (
      <div className="flex items-center justify-center min-h-screen bg-gray-100">
        <Spin size="large" />
      </div>
    );
  }

  if (!isAuthenticated) {
    return <Navigate to="/login" replace />;
  }

  return (
    <LayoutComponent>
      <Outlet />
    </LayoutComponent>
  );
};

export default PrivateRoute;
