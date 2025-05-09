locust -f load_test.py --host=http://localhost --headless -u 1000 -r 20 --run-time 3m --html=report.html
echo "Press Enter to continue..."
read