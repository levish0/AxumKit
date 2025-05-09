from locust import HttpUser, TaskSet, task, between

class UserBehavior(TaskSet):
    @task(1)
    def parse_test(self):
        self.client.get("http://127.0.0.1:8000/v0/user/1")


class User(HttpUser):
    tasks = [UserBehavior]
    wait_time = between(0, 10)  # seconds